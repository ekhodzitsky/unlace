# Verification boundary

The first source milestone uses Rust 1.89.0, safe Rust and no third-party Cargo
dependencies. The pinned toolchain is the minimum version used for local checks.
The example parses an EBML ID and an unknown size; it does not read a WebM file.

Integer tests cover every supported width, all prefixes of representative
headers, reserved/noncanonical IDs, sentinel sizes, nonminimal legal sizes,
the RFC 9559 0x80 update and exhaustive one/two-byte size inputs. Production
parsing uses bounded loops, does not allocate and returns typed errors.
No end-to-end demuxing, hostile container corpus or representative performance
comparison has passed yet because those capabilities are not implemented.

The dependency check reads locked offline Cargo metadata and rejects dependency
declarations in every category. Regular tests and rustdoc need no third-party
executables, downloaded fixtures or network. Local checks do not establish
Windows/macOS/ARM parity, fuzz coverage or whole-container conformance.

README standard v1 disposition: purpose, current example, alternatives and limits
are documented. Benchmark/coverage requirements remain incomplete; they must not
be replaced with invented numbers. Download/docs.rs indicators are inapplicable
to an unpublished private crate. Hosted CI is disabled. A distribution license
is pending owner selection. No production or universal-superiority claim is made.

Initial local gates passed on Linux with Rust 1.89.0: five integration tests and
one doctest in both debug and release, strict all-target Clippy, formatting,
warning-free rustdoc, the executable example and the zero-dependency check.
