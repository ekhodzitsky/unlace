# Container library with measured correctness and resource contracts

Status: accepted, 2026-11-01.

The owner selected unlace, private development and a goal of improving on the
strongest WebM/Matroska libraries. Implement a pure Rust container library without
third-party Cargo dependencies. Keep packet extraction independent of codecs,
networking and recognition. Begin with correct bounded reading; writing is a
separate capability. Do not copy a competitor's breadth without matching its
correctness and observable behavior on equivalent work.

[Design](../design.md) records the accepted contracts, [comparison](../comparison.md)
records primary evidence and [qualification](../qualification.md) defines how
performance and compatibility claims can be earned. The initial source provides
integer primitives only. No benchmark or whole-container support is claimed.

Consequence: private development, local checks and unpublished source milestones
continue until explicit publication authorization. A missing capability must be
reported honestly; it must not be hidden by silently accepting partial output.
No decision is superseded by this initial record.
