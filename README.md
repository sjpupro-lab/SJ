🔷 1️⃣ SHORT & STRONG (1-PAGE ENGLISH VERSION)

Canvapress™

Fixed-Packet Reversible Canvas Runtime

Canvapress is not a compression algorithm.

It is a fixed-size reversible container and runtime system
that maps arbitrary binary data into a deterministic 512×512 canvas.


---

What Makes It Different

Fixed Packet Size
Output size is deterministic.
No variable-length compression behavior.

Decode = Execution
Restoration is a state transition driven by integer arithmetic and bit operations.

Compression-State Usability
The canvas itself is structured data, not opaque compressed bytes.

Mathematically Reversible
Full trace convergence verification:

BA empty

RG restored to baseline




---

Performance (Python Reference)

Payload	Encode	Decode	Ratio

1 KB	0.07s	0.02s	0.30
10 KB	0.59s	0.18s	0.30
25 KB	1.42s	0.44s	0.31


Linear scaling

Decode faster than encode

100% reversible integrity



---

Capacity

With current RG constraints (3.125% of u64 range):

~1–2 TB theoretical reversible capacity


---

Use Cases

Deterministic network packets

Fixed-memory embedded systems

Obfuscated reversible containers

Runtime-executable binary formats



---

Core Principle

> Fixed canvas.
Deterministic arithmetic.
Reversible state machine.




---

🔷 2️⃣ BENCH + ARCHITECTURE VERSION (ENGLISH)

Canvapress Architecture Overview

System Flow

Input (bytes)
      │
      ▼
[ Encode Runtime ]
  - Integer arithmetic
  - Lane-based trace
  - Sparse BA step set
      │
      ▼
┌───────────────────────────┐
│ 512 × 512 Canvas Container│
│                           │
│  BA  → Sparse Step Set   │
│  R   → R Traces (u64)    │
│  G   → G Traces (u64)    │
│                           │
│  Fixed Packet Structure  │
└───────────────────────────┘
      │
      ▼
[ Decode Runtime ]
  - Step peel (N→1)
  - Bitset membership
  - Deterministic restore
      │
      ▼
Exact Output (bytes)


---

Runtime Model

Step Mapping

x = byte (0–255)

y = step & 511

pidx = (y << 9) + x


Lane Function

odd step → R

even step → G

k = ⌊step / 2⌋



---

Verification Condition

After full decode:

BA = empty

RG planes restored to baseline (FULL or 0 depending on profile)



---

Benchmarks (Reference Engine)

Payload	Encode(s)	Decode(s)	Decode/Encode

1,024	0.070	0.021	0.30
10,240	0.597	0.181	0.30
25,600	1.427	0.441	0.31


Observations:

Linear runtime scaling

Decode consistently faster

No entropy coder dependency

Integer-only arithmetic



---

🔷 3️⃣ 벤치 + 구조 한글판

Canvapress 구조 개요

전체 흐름

입력 바이트
      │
      ▼
[ 인코딩 런타임 ]
  - 정수 연산
  - R/G lane 규칙
  - Sparse BA 기록
      │
      ▼
┌────────────────────────────┐
│ 512 × 512 고정 캔버스      │
│                            │
│ BA → Step 집합(bitset)     │
│ R  → R 흔적(u64)           │
│ G  → G 흔적(u64)           │
│                            │
│ 고정 패킷 구조             │
└────────────────────────────┘
      │
      ▼
[ 디코딩 런타임 ]
  - step 역순 제거
  - 비트 연산 복원
  - 정수 상태 수렴
      │
      ▼
원본 복원


---

핵심 규칙

x = 바이트 값

y = step & 511

홀수 step → R

짝수 step → G

k는 step 증가에 따라 선형 증가



---

벤치 결과

Payload	Encode	Decode	비율

1KB	0.07s	0.02s	0.30
10KB	0.59s	0.18s	0.30
25KB	1.42s	0.44s	0.31


✔ 선형 증가
✔ 디코딩 더 빠름
✔ 완전 가역


---

차별성 요약

일반 압축:

가변 크기

엔트로피 기반

해제 전 사용 불가


Canvapress:

고정 패킷

가역 수학 런타임

압축 상태 활용 가능

실행 지향 구조