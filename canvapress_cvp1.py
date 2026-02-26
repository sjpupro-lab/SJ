#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Canvapress (CVP1) — 512×512 Fixed Packet Reversible Canvas Runtime
====================================================================

[고정 규칙 / 제약]
1) 포맷: 512×512 고정
2) 시간축: y = step & 511  (step은 1..N, 진행은 N→1로 통일)
3) 바이트값: x = byte (0..255)  → 바이트는 "좌표 x"
4) RG: 흔적(누산 벡터). step별 lane(R/G) + k를 더/빼는 가역 규칙
5) A(=BA): "이 픽셀이 밟은 step 집합"을 u64 비트셋(page,mask)로 저장 (sparse, 밟은 픽셀만 기록)
6) RG 제한: u64 최대의 3.125%만 사용
   RG_LIMIT = floor(2^64 / 32)
7) 메모리 접근(캐시): 인코딩/디코딩 모두 A→G/R 순으로 접근 (ABGR 컨셉)
8) step_to_x 같은 표를 RAW에 저장하지 않음.
   대신 RAW의 A(집합)에서 "step→pidx"를 1회 스캔으로 재구성(인메모리 캐시)하여 O(N) peel.

[RG 수식]
lane,k:
- step 홀수: lane=R, k=(step+1)//2
- step 짝수: lane=G, k=step//2

[인코딩(Erase) : N→1]
- 시작: RG는 FULL(=RG_LIMIT)로 가득 채움
- step=N..1:
    x = payload[step-1]
    y = step & 511
    pidx = y*512 + x
    A.set(pidx, step)      # 집합에 step 포함 기록 (A 먼저)
    RG[lane][pidx] -= k    # 흔적을 "지우며" 기록

[디코딩(Fill) : N→1]
- RAW 로드 후:
    A를 전체 스캔 1회 → step_to_pidx 캐시 생성 (파일 밖 인메모리)
- step=N..1:
    pidx = step_to_pidx[step]
    x = pidx & 511
    out[step-1] = x
    A.clear(pidx, step)    # A 먼저
    RG[lane][pidx] += k    # FULL로 복귀

[검증]
- 디코딩 종료 후:
    A는 empty
    RG는 FULL(RG_LIMIT)로 복귀

[RAW 파일 구성 (단일 파일로 복원 가능)]
- magic 4B: b'CVP1'
- u32: W(512)
- u32: H(512)
- u32: N (payload length)
- u64: RG_LIMIT
- R plane: u64 * (W*H)
- G plane: u64 * (W*H)
- u32: A_entry_count
- 반복 A entries:
    - u32: pidx
    - u16: page_count
    - 반복 pages:
        - u32: page
        - u64: mask
"""

from __future__ import annotations
from array import array
import random
import struct
import time
from typing import Dict, Tuple

W = 512
H = 512
PIXELS = W * H

RG_LIMIT = (1 << 64) // 32  # 3.125%
MAGIC = b"CVP1"


def lane_k(step: int) -> Tuple[int, int]:
    # 0=R, 1=G
    if step & 1:
        return 0, (step + 1) >> 1
    else:
        return 1, step >> 1


def pidx_of(x: int, y: int) -> int:
    return (y << 9) + x  # y*512 + x


class ABitset:
    """Sparse BA: pidx -> {page:u32 -> mask:u64}"""
    __slots__ = ("db",)

    def __init__(self):
        self.db: Dict[int, Dict[int, int]] = {}

    def set_step(self, pidx: int, step: int) -> None:
        page = step >> 6
        bit = step & 63
        m = 1 << bit
        d = self.db.get(pidx)
        if d is None:
            self.db[pidx] = {page: m}
        else:
            d[page] = d.get(page, 0) | m

    def clear_step(self, pidx: int, step: int) -> None:
        d = self.db.get(pidx)
        if d is None:
            raise ValueError("A mismatch: missing pidx")
        page = step >> 6
        bit = step & 63
        m = 1 << bit
        word = d.get(page, 0)
        if ((word >> bit) & 1) == 0:
            raise ValueError("A mismatch: bit already 0")
        word &= ~m
        if word == 0:
            d.pop(page, None)
            if not d:
                self.db.pop(pidx, None)
        else:
            d[page] = word

    def is_empty(self) -> bool:
        return len(self.db) == 0


class RGCanvas:
    __slots__ = ("R", "G")

    def __init__(self, full: bool):
        init = RG_LIMIT if full else 0
        self.R = array("Q", [init] * PIXELS)
        self.G = array("Q", [init] * PIXELS)


def build_step_index_from_A(A: ABitset, N: int) -> array:
    """
    step_to_x RAW 저장 금지 조건 충족:
    - A(집합)에서 step→pidx를 "1회 스캔"으로 재구성
    - 이후 peel은 O(N) 배열 접근
    """
    step_to_pidx = array("I", [0]) * (N + 1)
    seen = array("B", [0]) * (N + 1)

    for pidx, pages in A.db.items():
        for page, mask in pages.items():
            base = page << 6
            m = mask
            while m:
                lsb = m & -m
                bit = lsb.bit_length() - 1
                step = base + bit
                if 1 <= step <= N:
                    if seen[step]:
                        raise ValueError(f"step collision in A: step={step}")
                    seen[step] = 1
                    step_to_pidx[step] = pidx
                m ^= lsb

    for s in range(1, N + 1):
        if not seen[s]:
            raise ValueError(f"missing step in A: step={s}")

    return step_to_pidx


def raw_pack(rg: RGCanvas, A: ABitset, N: int) -> bytes:
    buf = bytearray()
    buf += MAGIC
    buf += struct.pack("<IIIQ", W, H, N, RG_LIMIT)
    buf += rg.R.tobytes()
    buf += rg.G.tobytes()

    buf += struct.pack("<I", len(A.db))
    for pidx, pages in A.db.items():
        buf += struct.pack("<IH", pidx, len(pages))
        for page, mask in pages.items():
            buf += struct.pack("<IQ", page, mask)
    return bytes(buf)


def raw_unpack(raw_bytes: bytes) -> Tuple[RGCanvas, ABitset, int]:
    mv = memoryview(raw_bytes)
    off = 0

    if mv[off:off+4].tobytes() != MAGIC:
        raise ValueError("bad magic")
    off += 4

    w, h, N, rg_limit = struct.unpack_from("<IIIQ", mv, off)
    off += struct.calcsize("<IIIQ")

    if w != W or h != H:
        raise ValueError(f"bad dims: {w}x{h}")
    if rg_limit != RG_LIMIT:
        raise ValueError("RG_LIMIT mismatch")

    rg = RGCanvas(full=False)
    A = ABitset()

    plane_bytes = 8 * PIXELS

    rg.R = array("Q")
    rg.R.frombytes(mv[off:off+plane_bytes].tobytes())
    off += plane_bytes

    rg.G = array("Q")
    rg.G.frombytes(mv[off:off+plane_bytes].tobytes())
    off += plane_bytes

    (cnt,) = struct.unpack_from("<I", mv, off)
    off += 4

    for _ in range(cnt):
        pidx, pcnt = struct.unpack_from("<IH", mv, off)
        off += struct.calcsize("<IH")
        pages: Dict[int, int] = {}
        for _ in range(pcnt):
            page, mask = struct.unpack_from("<IQ", mv, off)
            off += struct.calcsize("<IQ")
            pages[page] = mask
        A.db[pidx] = pages

    return rg, A, N


def encode_erase(payload: bytes) -> bytes:
    N = len(payload)
    if N <= 0:
        raise ValueError("empty payload")

    rg = RGCanvas(full=True)
    A = ABitset()
    R, G = rg.R, rg.G

    for step in range(N, 0, -1):
        x = payload[step - 1]
        y = step & 511
        pidx = pidx_of(x, y)

        A.set_step(pidx, step)  # A 먼저

        lane, k = lane_k(step)
        if lane == 0:
            v = R[pidx]
            if v < k: raise ValueError("R underflow encode")
            R[pidx] = v - k
        else:
            v = G[pidx]
            if v < k: raise ValueError("G underflow encode")
            G[pidx] = v - k

    return raw_pack(rg, A, N)


def decode_fill(raw_bytes: bytes) -> bytes:
    rg, A, N = raw_unpack(raw_bytes)

    step_to_pidx = build_step_index_from_A(A, N)

    out = bytearray(N)
    R, G = rg.R, rg.G

    for step in range(N, 0, -1):
        pidx = step_to_pidx[step]
        x = pidx & 511
        out[step - 1] = x

        A.clear_step(pidx, step)  # A 먼저

        lane, k = lane_k(step)
        if lane == 0:
            v = R[pidx] + k
            if v > RG_LIMIT: raise ValueError("R overflow decode")
            R[pidx] = v
        else:
            v = G[pidx] + k
            if v > RG_LIMIT: raise ValueError("G overflow decode")
            G[pidx] = v

    if not A.is_empty():
        raise ValueError("A not empty after decode")
    if any(v != RG_LIMIT for v in rg.R) or any(v != RG_LIMIT for v in rg.G):
        raise ValueError("RG not FULL after decode")

    return bytes(out)


def bench_once(n: int, seed: int = 7) -> None:
    rnd = random.Random(seed)
    payload = bytes(rnd.randrange(256) for _ in range(n))

    t0 = time.time()
    raw = encode_erase(payload)
    t1 = time.time()
    restored = decode_fill(raw)
    t2 = time.time()

    inj = t1 - t0
    ext = t2 - t1

    print(f"payload: {n}")
    print(f"encode(erase): {inj:.6f}")
    print(f"decode(fill) : {ext:.6f}")
    print(f"ratio        : {(ext/inj):.3f}" if inj > 0 else "ratio        : inf")
    print(f"match        : {restored == payload}")
    print("-" * 44)


if __name__ == "__main__":
    bench_once(1024)
    bench_once(10240)
    bench_once(25600)