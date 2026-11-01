# Incremental EBML framing

The unreleased `stream` module frames EBML elements over `std::io::Read` without
requiring `Seek`. It is a reusable structural core. It does not yet select a
Matroska/WebM schema, validate an EBML document or interpret track/block metadata.
The executable `stream_ebml` example frames a synthetic EBML header.

Supply a trusted table of `ElementSpec` entries through `Schema::new`. Each ID
has one nonrecursive parent path or is global. Schema validation checks canonical
IDs, unique entries, master parents, acyclic paths and unknown-size permissions
on ancestors. This subset does not express recursive Matroska chapters, scalar
value types, minimum/maximum occurrence counts, ordering, defaults or CRC checks.
The application must enforce those semantics before claiming document validity.
Global entries apply at root and inside every master; restricted globals such
as CRC-32 require additional application validation.

`Reader::poll` emits `Start`, `Data` and `End` for each interpreted leaf, and
`Start` and `End` for masters and opaque skipped elements. Data occupies the
first `len` bytes of the caller's nonempty buffer, which the reader never retains.
The reader owns its source and continuation state. There is no checkpoint import
or arbitrary resynchronization. `position` counts consumed source bytes, including
lookahead headers; event headers report their own offsets from the initial source
position. `into_inner` recovers the source but discards that continuation state.

## Boundaries and skipping

Unknown-size handling follows [RFC 8794 section 6.2](https://www.rfc-editor.org/rfc/rfc8794.html#section-6.2).
Only explicitly permitted masters may have unknown size. A valid next sibling,
ancestor or root can end an unknown-sized master; globals remain children of the
current master. Known finite parent boundaries also terminate unknown children.
An incoming element cannot escape an unfinished finite parent. All finite child
ends use checked offset arithmetic and must fit the nearest finite boundary.
The reader never searches compressed payload bytes for element ID patterns.

Unknown IDs return `UnsupportedId` even with a finite size: inferring a parent or
unknown-size termination from an unrecognized ID would be speculative. To ignore
a known finite element, declare `Kind::Skip` at its proper parent. Its outer
boundary is checked and its bytes are consumed in bounded chunks; its contents
are opaque. This can skip a known finite master, but its descendants are not
validated. Unknown-size skipped elements are refused. A caller needing validated
children must declare the master and its children instead.

## Input states

| Result | Contract |
| --- | --- |
| `NeedInput` | Source returned `WouldBlock`; header/payload state is retained exactly. Resume with the same reader. |
| `Yield` | Per-poll read-call or byte budget was reached. Resume with the same reader. |
| `SourceEof` | Source returned zero at a structural boundary after all End events. Repeated polling returns the same event. |
| `Truncated` | Source returned zero inside a header, leaf or unfinished finite master. |
| `Integer`, `Invalid` | Invalid integer syntax or structural relationship/boundary. |
| `UnsupportedId` | The supplied schema cannot interpret this element. |
| `Limit` | Refused declared metadata/payload/skip size, depth or integer width. |
| `Io` | Other source error, preserving the underlying error. |
| `Configuration` | Invalid construction limits or empty output buffer; an empty-buffer poll consumes nothing and may be retried. |
| `Failed` | A previous terminal error poisoned continuation. |

`Read::read` returning zero is final EOF. A temporarily exhausted producer must
return `WouldBlock`, including when pausing inside a header. Interrupted reads
are retried only within the current call budget. Source EOF cannot prove that an
unknown-sized stream contains all packets intended by its producer. End events
identify whether a master closed at a boundary, next element or source EOF.
Even finite structural completion does not establish required document contents,
source identity or codec/audio completeness. Empty input is a structural EOF,
not a valid EBML document.

## Resource contract

There are no reader allocations. Retained storage consists of a twelve-byte
header, at most 64 fixed master slots, one active leaf and one lookahead header.
The configured maximum depth can be smaller. Caller buffer size controls payload
chunk storage. Metadata and payload limits apply to each declared leaf; skip has
its own finite-size limit. The reader accumulates no metadata or index, so callers
retaining multiple values must separately bound their aggregate storage.

Finite master sizes are not allocations and are not capped by recording length.
There is no lifetime byte, element or packet counter limit other than checked
`u64` offsets. Work limits reset per poll. Header parsing takes at most twelve
bytes; a poll returns after one observable event, one bounded payload chunk, or
budget exhaustion. Even uninterrupted opaque skipping and repeated `Interrupted`
errors yield within those budgets. Schema lookup cost depends on the trusted
table, not adversarial input size. Budgets do not bound latency or work performed
inside an implementation of `Read`; blocking sources can block inside a call.

Default limits are 16 open masters, 1 MiB per metadata leaf, 16 MiB per payload
leaf, 64 MiB per skipped element, 32 read calls and 64 KiB per poll, four ID bytes
and eight size bytes. `Limits` makes each explicit. Width limits can be supplied
from document declarations by a higher layer; the framing core does not itself
interpret or update them from an EBML header. ID syntax incorporates the
[RFC 9559 section 4.2 correction](https://www.rfc-editor.org/rfc/rfc9559.html#section-4.2).
