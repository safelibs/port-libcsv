# libcsv Rust port documentation

## High-level architecture

This checkout is a one-member Cargo workspace. `Cargo.toml:1-6` declares only the `safe` member and sets the release profile to `panic = "abort"`, which is why `safe/src/lib.rs:1-11` can switch the library into `no_std` mode for release artifacts and install release-only allocator and panic machinery at `safe/src/lib.rs:55-155`.

The crate boundary is small and explicit.

- `safe/Cargo.toml:1-9` declares one library crate named `csv` with crate types `cdylib`, `staticlib`, and `rlib`. It declares no `[dependencies]`, no `[build-dependencies]`, no Cargo features, and no binaries.
- `safe/src/lib.rs:13-165` glues the crate together. It exposes the public constants, wires the `engine`, `ffi`, and `rust_api` modules together, and in release `panic = "abort"` builds installs the libc-backed allocator and panic-abort hooks at `safe/src/lib.rs:55-155`.
- `safe/src/engine.rs:9-627` is the safe parser and writer engine used by Rust callers. It owns the parser state machine, safe buffer growth through `Vec<u8>`, writer quoting helpers, and the unit tests in `safe/src/engine.rs:629-774`.
- `safe/src/rust_api.rs:14-127` is the Rust-native facade. It exposes `Parser`, safe callback-based parsing, and writer helpers for direct Rust callers and the pure-Rust tests.
- `safe/src/ffi.rs:196-938` is the public C ABI boundary. It defines `csv_parser`, validates pointers, dispatches callbacks, grows the C-visible entry buffer, exposes the exported `csv_*` functions, and contains the Linux x86_64 `global_asm!` trampolines at `safe/src/ffi.rs:14-193` that attach `@@Base` symbol versions to those exports.
- `safe/build.rs:3-13` and `safe/libcsv.map:1-27` finish the shared-library ABI setup. `safe/build.rs:9-12` passes the linker version script and the `libcsv.so.3` SONAME, and `safe/libcsv.map:1-27` limits the exported symbol set to the 22 public `csv_*` names.
- `safe/debian/rules:14-35` is manual install glue. It builds the release artifacts, installs `target/release/libcsv.so` and `target/release/libcsv.a`, installs the shipped public header from `original/csv.h`, installs the upstream manpage from `original/csv.3`, and copies the upstream example sources from `original/examples/`.

There is no generated-header flow in this port. Outside `safe/PORT.md`, a repository-wide search for `cbindgen` and `bindgen` returned no matches, `safe/build.rs` only injects linker arguments, and the shipped header is the upstream file `original/csv.h:1-88`.

The two symbol files serve different purposes and should not be conflated.

- `safe/debian/libcsv3.symbols:1-24` is the packaging copy that would ship in the Debianized Rust port.
- `original/debian/libcsv3.symbols:1-24` is the upstream comparison copy used by the ABI regression test in `safe/tests/c_abi.rs:192-204` and `safe/tests/c_abi.rs:239-266`.

Directory map for the port and its comparison points:

- `safe/Cargo.toml`: crate manifest for the Rust port.
- `safe/build.rs`: linker glue for the SONAME and version script.
- `safe/libcsv.map`: authoritative export list for the shared object.
- `safe/src/`: Rust implementation, split into `lib.rs`, `engine.rs`, `ffi.rs`, and `rust_api.rs`.
- `safe/debian/`: packaging metadata, install rules, symbols file, and autopkgtest metadata.
- `safe/tests/`: Rust, C, and mixed-language regression tests.
- `original/csv.h`: shipped public header and ABI/layout comparison source.
- `original/libcsv.c`: upstream C implementation used as the behavioral reference.
- `original/test_csv.c`: upstream C regression test that `safe/tests/c_compat.rs:239-262` compiles against the Rust artifacts.
- `original/examples/`: upstream example C programs that `safe/tests/c_compat.rs:196-291` compiles and runs against the Rust artifacts.
- `original/debian/libcsv3.symbols`: upstream export list used as the ABI comparison source.

## Where the unsafe Rust lives

The mechanical `unsafe` scan excluded generated documentation such as this file and found `unsafe` only in `safe/src/lib.rs`, `safe/src/ffi.rs`, `safe/tests/c_abi.rs`, and `safe/tests/helpers/panic_tripwire.rs`. `safe/src/engine.rs`, `safe/src/rust_api.rs`, `safe/tests/public_api.rs`, `safe/tests/original.rs`, `safe/tests/c_compat.rs`, `safe/tests/downstream_regressions.rs`, and `safe/tests/panic_boundary.rs` contain no `unsafe` tokens.

- `safe/src/lib.rs:56-63` declares raw libc entry points for the release-only allocator and abort path. `safe/src/lib.rs:69-100` is an `unsafe impl GlobalAlloc` that calls `malloc`, `calloc`, `free`, and `realloc`, and also performs `ptr::write_bytes`. `safe/src/lib.rs:108-110` aborts on panic. `safe/src/lib.rs:126-155` contains `unsafe fn aligned_malloc` and `unsafe fn realloc_fallback`, using `posix_memalign`, `Layout::from_size_align_unchecked`, `allocator.alloc`, and `ptr::copy_nonoverlapping`.
- `safe/src/ffi.rs:196-206` marks the C callback and allocator function-pointer types as `unsafe extern "C"`. `safe/src/ffi.rs:251-256` wraps raw libc `realloc` and `free`. `safe/src/ffi.rs:261`, `279`, and `291` abort on invalid pointers and turn raw pointers into slices. `safe/src/ffi.rs:303-406` is the internal unsafe core for predicate calls, C-buffer growth, pointer writes, and callback dispatch.
- Most exported ABI functions in `safe/src/ffi.rs` are raw-pointer entry points and therefore remain `unsafe extern "C"`: `csv_error` at `safe/src/ffi.rs:428-429`, `csv_get_opts` at `safe/src/ffi.rs:444-448`, `csv_set_opts` at `safe/src/ffi.rs:452-456`, `csv_init` at `safe/src/ffi.rs:461-467`, `csv_free` at `safe/src/ffi.rs:488-500`, `csv_fini` at `safe/src/ffi.rs:507-582`, the delimiter/quote/predicate/allocator/block-size getters and setters at `safe/src/ffi.rs:585-640`, the main parser at `safe/src/ffi.rs:645-883`, and the writer wrappers at `safe/src/ffi.rs:886-938`. `csv_strerror` at `safe/src/ffi.rs:432-442` is the exception: it is exported as plain `extern "C"`, takes only a status code, and does not require an `unsafe` call site.
- `safe/tests/c_abi.rs:25-32` declares test-only libc entry points. `safe/tests/c_abi.rs:43-50`, `53-59`, and `61-84` define test-only allocator hooks, predicate callbacks, and field/row callbacks that dereference raw pointers, mutate the live entry buffer, and convert raw bytes to slices. The integration tests continue using unsafe calls at `safe/tests/c_abi.rs:273-447` to zero-initialize `csv_parser`, call the exported `csv_*` ABI, exercise `FILE *` I/O, and turn `csv_strerror` pointers into `CStr`.
- `safe/tests/helpers/panic_tripwire.rs:15-20` contains helper `unsafe extern "C"` function-pointer fields in the local `csv_parser` mirror. `safe/tests/helpers/panic_tripwire.rs:24-41` declares linked `csv_*` ABI entry points, `safe/tests/helpers/panic_tripwire.rs:43-45` defines an unsafe callback that panics, and `safe/tests/helpers/panic_tripwire.rs:48-67` zero-initializes a parser and calls the ABI from a separate process.

Unsafe that is not strictly required by the public C ABI is limited and easy to identify.

- All of `safe/src/lib.rs:55-155` is release-only allocator and panic machinery driven by the workspace release profile, not by the public `csv_*` ABI itself.
- All unsafe in `safe/tests/c_abi.rs` and `safe/tests/helpers/panic_tripwire.rs` is non-shipped test harness code used to validate the ABI and panic boundary.
- The intended shipped ABI-facing unsafe is concentrated in `safe/src/ffi.rs`.

## Remaining unsafe FFI beyond the original ABI/API boundary

This section separates the intended `csv_*` surface from the extra FFI that still exists around it. The shared-object checks were `readelf --dyn-syms --wide target/release/libcsv.so`, `readelf -Ws --wide target/release/libcsv.so`, `readelf -V target/release/libcsv.so`, and `nm -D --undefined-only target/release/libcsv.so`. The static-archive checks were `nm -u target/release/libcsv.a`, `ar t target/release/libcsv.a`, and an extracted-member `nm -u` pass over the relevant archive members. Shared-object inspection matters here because not every source-declared extern survives into `target/release/libcsv.so`: in this checkout the `.so` leaves only `abort`, `free`, `realloc`, and `fputc` unresolved, while `calloc`, `malloc`, and `posix_memalign` stay archive-only. I do not count linker-generated weak undefined symbols such as `__gmon_start__`, `_ITM_*`, and `__cxa_finalize` as deliberate handwritten FFI.

- Intended public libcsv C ABI surface: the 22 `csv_*` symbols listed in `safe/libcsv.map:1-27`, declared by `original/csv.h:61-82`, and implemented in `safe/src/ffi.rs:428-938` are the preserved public ABI. Provider: this crate itself. Why they exist: they are the libcsv API that downstream C callers link against. Artifact scope: `readelf --dyn-syms --wide target/release/libcsv.so` shows exactly those 22 `@@Base` exports, and `safe/tests/c_abi.rs:239-266` checks that export set against `original/debian/libcsv3.symbols:1-24`. Safe-Rust replacement: none, because removing or renaming them would break the port's purpose.
- Additional production FFI, `abort`: declaration sites `safe/src/ffi.rs:244-245` and `safe/src/lib.rs:56-57`. Provider: glibc/libc. Why it exists: `safe/src/ffi.rs:260-268` aborts on invalid mandatory pointers, and `safe/src/lib.rs:108-123` aborts on panic or synthetic personality entry in release `panic = "abort"` builds. Artifact scope: present in both `target/release/libcsv.so` and `target/release/libcsv.a`. Safe-Rust replacement: `std::process::abort()` would only be possible if the crate stopped being `no_std` in this configuration, so there is no current safe-Rust replacement in the shipped build.
- Additional production FFI, C-ABI-side `free` and `realloc`: declaration site `safe/src/ffi.rs:246-248`. Provider: glibc/libc. Why they exist: `safe/src/ffi.rs:251-256` installs the default buffer hooks, `safe/src/ffi.rs:317-353` grows the C-visible entry buffer through `realloc`, and `safe/src/ffi.rs:488-500` frees that buffer through the configured free hook. Artifact scope: present in both `target/release/libcsv.so` and `target/release/libcsv.a`. Safe-Rust replacement: there is no practical safe-Rust replacement while preserving the C-visible mutable entry buffer and the `csv_set_realloc_func` and `csv_set_free_func` behavior; switching to pure `Vec<u8>` ownership would change observable ABI semantics.
- Additional production FFI, allocator-side `free` and `realloc`: declaration sites `safe/src/lib.rs:59` and `safe/src/lib.rs:62`. Provider: glibc/libc. Why they exist: `safe/src/lib.rs:90-98` implements `LibcAllocator::dealloc` and `LibcAllocator::realloc` in terms of libc, and `safe/src/lib.rs:148-150` frees the old allocation during the aligned `realloc_fallback` path. Artifact scope: both symbol names appear in `target/release/libcsv.a`; the same unresolved names also remain in `target/release/libcsv.so`, but shared-object inspection cannot separate this allocator path from the `safe/src/ffi.rs:246-248` path because both compile down to the same libc symbol names. Safe-Rust replacement: `std::alloc::System`, or removing the custom allocator entirely, would be the most plausible safe-Rust replacement if the port accepted `std` in release builds.
- Additional production FFI, `fputc`: declaration site `safe/src/ffi.rs:247` and use site `safe/src/ffi.rs:920-933`. Provider: glibc/libc. Why it exists: `csv_fwrite` and `csv_fwrite2` must write quoted bytes to a caller-supplied `FILE *`, which is part of the original API in `original/csv.h:68-70`. Artifact scope: present in both `target/release/libcsv.so` and `target/release/libcsv.a`. Safe-Rust replacement: the Rust-native API already has one at `safe/src/rust_api.rs:116-127` via `Write`, but that cannot replace the shipped `FILE *` ABI without breaking compatibility.
- Additional production FFI, `calloc`, `malloc`, and `posix_memalign`: declaration sites `safe/src/lib.rs:58-61`, with use sites in `safe/src/lib.rs:70-84` and `safe/src/lib.rs:126-135`. Provider: glibc/libc. Why they exist: the release-only `LibcAllocator` uses libc allocation routines so the `staticlib` and `rlib` builds can stay in the `panic = "abort"` and `no_std` configuration without relying on `std::alloc::System`. Artifact scope: these names appear in `nm -u target/release/libcsv.a`, but they do not appear in `readelf -Ws --wide target/release/libcsv.so`, so they are static-archive-only in this checkout. Safe-Rust replacement: `std::alloc::System` would be the most plausible replacement if the port accepted a `std` dependency in release builds.
- Static-archive-only unresolved references, `memcpy` and `memset`: source sites `safe/src/lib.rs:84` and `safe/src/lib.rs:148-150`. Provider: glibc/libc or the linker-selected libc implementation. Why they exist: the allocator fallback zeroes and copies raw memory when aligned reallocation cannot delegate directly to libc `realloc`. Artifact scope: present in `nm -u target/release/libcsv.a` and absent from the shared-object undefined set. Safe-Rust replacement: the source could be rewritten with safe slice operations, but the compiler may still lower those operations to the same libc helpers.
- Static-archive-only unresolved references, Rust runtime internals `_RNvCs5QKde7ScR4H_7___rustc35___rust_no_alloc_shim_is_unstable_v2`, `_ZN4core5slice5index16slice_index_fail17h172ece6e023ae9aaE`, `_ZN5alloc7raw_vec12handle_error17h36dee3f3cfdaa106E`, and `_RNvCs5QKde7ScR4H_7___rustc25___rdl_alloc_error_handler`: these were observed in the extracted archive members `csv.csv.b2eec0a251896c4a-cgu.0.rcgu.o` and `csv.demqohd0nao5offu0zrrl7e7x.rcgu.o`, which are the compiled form of the Rust crate sources. Provider: the Rust `core` and `alloc` runtime support objects. Why they exist: the static archive keeps Rust allocation and bounds-check failure edges available for static links. Artifact scope: `target/release/libcsv.a` only. Safe-Rust replacement: none at the source level beyond changing the allocator, panic, and bounds-check strategy.
- Static-archive-only unresolved references, compiler runtime internals `__compilerrt_abort_impl` and `__udivti3`: these were observed in extracted archive members `45c91108d938afe8-absvdi2.o` and `45c91108d938afe8-mulvti3.o`. Provider: `compiler_builtins` or compiler-rt support objects bundled into the Rust static archive. Why they exist: they are toolchain-supplied arithmetic and overflow helpers, not handwritten FFI in the port. Artifact scope: `target/release/libcsv.a` only. Safe-Rust replacement: none at the source level.
- Test-only FFI, libc plus `csv_*` linkage in `safe/tests/c_abi.rs:25-32` and `safe/tests/c_abi.rs:61-84,273-447`: providers are glibc/libc and this crate under test. Why it exists: the ABI test needs real `FILE *` operations, custom allocator hooks, raw callback pointers, and direct calls into the exported C ABI. Artifact scope: tests only; nothing here ships in `libcsv.so` or `libcsv.a`. Safe-Rust replacement: some file handling could be written in pure Rust, but that would stop exercising the real `FILE *` surface and would weaken the ABI check.
- Test-only FFI, `#[link(name = "csv")]` and `csv_*` externs in `safe/tests/helpers/panic_tripwire.rs:23-41`, plus the callback at `safe/tests/helpers/panic_tripwire.rs:43-67`: provider is this crate under test. Why it exists: the panic-boundary test must cross a real C ABI in a separate process to prove that panic does not unwind across the boundary. Artifact scope: tests only. Safe-Rust replacement: none if the goal remains a real cross-language abort check.

## Remaining issues

The current verification evidence is good, but it is still evidence, not a formal proof.

- `cargo test --manifest-path safe/Cargo.toml` passed in this pass. That covered the unit tests in `safe/src/engine.rs:700-773`, the Rust public API tests in `safe/tests/public_api.rs:76-229`, the translated upstream behavior suite in `safe/tests/original.rs:123-770`, the ABI and export checks in `safe/tests/c_abi.rs:233-451`, the compiled-C compatibility checks in `safe/tests/c_compat.rs:222-291`, the downstream fixture regressions in `safe/tests/downstream_regressions.rs:51-158`, and the cross-process abort check in `safe/tests/panic_boundary.rs:75-100`.
- Debian package builds intentionally skip tests. `safe/debian/rules:17-18` overrides `dh_auto_test` with a no-op, so a successful Debian package build is not by itself test evidence.
- The scoped issue-marker scan over `safe/src`, `safe/tests`, `safe/debian`, `safe/build.rs`, `safe/Cargo.toml`, and `safe/libcsv.map` found no `TODO` or `FIXME` markers. I did not treat comments elsewhere in `original/` as port issues because they are not shipped Rust-port metadata.
- Semantics-level and bit-for-bit evidence is specific, not global. `safe/tests/original.rs:123-770` translates the behavior asserted by `original/test_csv.c:12-174` into Rust-native parser and writer checks. `safe/tests/c_compat.rs:239-262` goes further by compiling `original/test_csv.c` itself against both `target/release/libcsv.so` and `target/release/libcsv.a` and asserting the original `"All tests passed\n"` output, while `safe/tests/c_compat.rs:89-99` also stages `target/compat/include/csv.h` as a byte-for-byte copy of `original/csv.h`. `safe/tests/c_compat.rs:196-291` compiles and runs the upstream examples from `original/examples/` and additional C fixtures. `safe/tests/c_abi.rs:107-230` and `safe/tests/c_abi.rs:239-266` prove struct layout, SONAME, and export-list equivalence against `original/csv.h` and `original/debian/libcsv3.symbols`. `safe/tests/downstream_regressions.rs:51-128` pins the currently known readstat, csvcheck, and csvvalid fixture semantics. The remaining caveat is that any behavior not covered by those tests is still unproven in this pass.
- The authoritative direct-dependent inventory is `dependents.json:1-69`, which currently lists only `readstat` and `Tellico` as direct Ubuntu noble dependents. `downstream-apps.json:1-91` and the scripts under `downstream/apps/` are a broader harness inventory used for matrix testing; they are not the authoritative dependency census.
- The authoritative CVE input is `relevant_cves.json:1-34`. Its current summary is zero published relevant CVEs, with only one non-CVE note for `rgamble/libcsv#29`.
- `downstream-findings.json:1-57` records resolved historical harness gaps, but I did not rerun the full Docker-based downstream matrices in this pass.
- The only downstream-matrix command exercised in this pass was `python3 scripts/downstream-matrix.py validate --inventory downstream-apps.json`, triggered by `safe/tests/downstream_regressions.rs:144-151` during `cargo test --manifest-path safe/Cargo.toml`. I did not run `python3 scripts/downstream-matrix.py run ...`, `test-downstream.sh`, or any Docker-backed downstream execution path.
- `test-original.sh` and `test-downstream.sh` were not run in this pass. The compatibility evidence therefore comes from `cargo test --manifest-path safe/Cargo.toml` and the checked-in inventories and fixtures, not from the optional Docker-based end-to-end harnesses.
- No benchmark or throughput claims were made in this pass. I did not take new performance measurements, so there is no evidence for performance parity with the C implementation.

## Dependencies and other libraries used

`safe/Cargo.toml:1-9` has no `[dependencies]`, no `[build-dependencies]`, and no proc-macro dependencies. The port does not pull in any third-party Cargo crates at manifest level.

- Runtime and system-library dependencies inferred from the built artifacts: the Debian runtime package metadata in `safe/debian/control:17-19` depends on `libc6`, and the shared object still expects libc-provided `abort`, `free`, `realloc`, and `fputc` according to `nm -D --undefined-only target/release/libcsv.so` and `readelf -Ws --wide target/release/libcsv.so`. The shared object in this checkout has no explicit `DT_NEEDED` entries in `readelf -d target/release/libcsv.so`, so those libc symbols are visible as unresolved references rather than as a recorded shared-library dependency. The static archive can additionally require `calloc`, `malloc`, `posix_memalign`, `memcpy`, `memset`, Rust runtime support, and compiler runtime helpers according to `nm -u target/release/libcsv.a`; those are static-link concerns, not Cargo manifest dependencies.
- Build-time Debian dependencies from `safe/debian/control:1-12`: `debhelper-compat (= 13)`, `cargo`, and `rustc`.
- Test and autopkgtest dependencies from `safe/debian/tests/control:1-3`: `build-essential` and `libcsv-dev`.
- Local tool dependencies used to produce this document: `git`, `rg`, `nl`, `sed`, `cargo`, `rustc`, `readelf`, `nm`, `ar`, `cc` or `gcc`, `cmp`, and `python3`. `cargo test --manifest-path safe/Cargo.toml` specifically exercised test code that shells out to `readelf`, `cc` or `gcc`, `rustc`, and `python3`.
- Toolchain crates such as `core`, `alloc`, and `compiler_builtins` appear only as implementation details of the release and static-link artifacts seen in `target/release/libcsv.a`; they are not Cargo manifest dependencies for this port.

## How this document was produced

Exact commands run during this pass, including the downstream-matrix validator subprocess that `cargo test` exercised:

```sh
git status --short
nl -ba Cargo.toml
nl -ba safe/Cargo.toml
nl -ba safe/build.rs
nl -ba safe/libcsv.map
nl -ba safe/src/lib.rs
nl -ba safe/src/engine.rs
rg -n "unsafe|extern \"C\"|global_asm|csv_[A-Za-z0-9_]+" safe/src/ffi.rs
nl -ba safe/src/ffi.rs | sed -n '1,260p'
nl -ba safe/src/ffi.rs | sed -n '260,520p'
nl -ba safe/src/ffi.rs | sed -n '520,980p'
nl -ba safe/src/rust_api.rs
rg -n "unsafe|extern \"C\"|csv_[A-Za-z0-9_]+|libc::|std::mem::zeroed|from_raw_parts|from_raw_parts_mut" safe/tests safe/src
nl -ba safe/tests/c_abi.rs | sed -n '1,520p'
nl -ba safe/tests/helpers/panic_tripwire.rs
nl -ba safe/tests/original.rs
nl -ba safe/tests/c_compat.rs
nl -ba safe/tests/downstream_regressions.rs
nl -ba safe/tests/panic_boundary.rs
nl -ba safe/tests/public_api.rs
nl -ba safe/debian/control
nl -ba safe/debian/rules
nl -ba safe/debian/libcsv3.symbols
nl -ba original/debian/libcsv3.symbols
nl -ba safe/debian/tests/control
nl -ba safe/debian/tests/build-examples
sed -n '1,200p' dependents.json
sed -n '1,220p' relevant_cves.json
sed -n '1,260p' downstream-findings.json
sed -n '1,260p' downstream-apps.json
nl -ba test-original.sh
nl -ba test-downstream.sh
nl -ba original/csv.h | sed -n '1,220p'
rg --files original/examples safe/src safe/tests safe/debian
nl -ba scripts/downstream-matrix.py | sed -n '1,220p'
rg --files downstream/apps downstream/fixtures
cargo build --manifest-path safe/Cargo.toml --release
readelf -d target/release/libcsv.so
readelf --dyn-syms --wide target/release/libcsv.so
readelf -Ws --wide target/release/libcsv.so
nm -D --undefined-only target/release/libcsv.so
nm -u target/release/libcsv.a
command -v llvm-nm || true
ar t target/release/libcsv.a | sed -n '1,80p'
tmpdir=$(mktemp -d) && cd "$tmpdir" && ar x /home/yans/safelibs/pipeline/ports/port-libcsv/target/release/libcsv.a csv.csv.b2eec0a251896c4a-cgu.0.rcgu.o csv.demqohd0nao5offu0zrrl7e7x.rcgu.o 45c91108d938afe8-mulvti3.o 45c91108d938afe8-absvdi2.o && echo TMP:$tmpdir && nm -u csv.csv.b2eec0a251896c4a-cgu.0.rcgu.o && echo '---' && nm -u csv.demqohd0nao5offu0zrrl7e7x.rcgu.o && echo '---' && nm -u 45c91108d938afe8-mulvti3.o && echo '---' && nm -u 45c91108d938afe8-absvdi2.o
cargo test --manifest-path safe/Cargo.toml
python3 scripts/downstream-matrix.py validate --inventory downstream-apps.json
cargo geiger --version
rg -n '\bunsafe\b' safe/src/lib.rs safe/src/ffi.rs safe/tests/c_abi.rs safe/tests/helpers/panic_tripwire.rs
rg -n 'TODO|FIXME' safe/src safe/tests safe/debian safe/build.rs safe/Cargo.toml safe/libcsv.map
if [ -f safe/PORT.md ]; then echo exists; nl -ba safe/PORT.md | sed -n '1,260p'; else echo missing; fi
nl -ba original/libcsv.c | sed -n '1,220p'
nl -ba original/test_csv.c | sed -n '1,220p'
rg -n 'cbindgen|bindgen' .
ldd target/release/libcsv.so
readelf -V target/release/libcsv.so
if [ -f target/compat/include/csv.h ]; then cmp -s original/csv.h target/compat/include/csv.h && echo match; else echo missing; fi
```

`cargo geiger` was unavailable in this environment: `cargo geiger --version` failed with `error: no such command: geiger`, so no Geiger report was used.

Starting cleanliness baseline from `git status --short`:

```text
[no output]
```

The worktree was clean before any edits in this pass.

Files whose contents I opened or otherwise directly inspected:

- `Cargo.toml`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/libcsv.map`
- `safe/src/lib.rs`
- `safe/src/engine.rs`
- `safe/src/ffi.rs`
- `safe/src/rust_api.rs`
- `safe/debian/control`
- `safe/debian/rules`
- `safe/debian/libcsv3.symbols`
- `safe/debian/tests/control`
- `safe/debian/tests/build-examples`
- `safe/tests/public_api.rs`
- `safe/tests/original.rs`
- `safe/tests/c_abi.rs`
- `safe/tests/c_compat.rs`
- `safe/tests/downstream_regressions.rs`
- `safe/tests/panic_boundary.rs`
- `safe/tests/helpers/panic_tripwire.rs`
- `original/csv.h`
- `original/libcsv.c`
- `original/test_csv.c`
- `original/debian/libcsv3.symbols`
- `dependents.json`
- `relevant_cves.json`
- `downstream-findings.json`
- `downstream-apps.json`
- `scripts/downstream-matrix.py`
- `test-original.sh`
- `test-downstream.sh`
- `target/release/libcsv.so`
- `target/release/libcsv.a`
- `target/compat/include/csv.h`

Paths enumerated as prepared fixtures or harness inventory during this pass:

- `original/examples/Makefile`
- `original/examples/csvfix.c`
- `original/examples/csvinfo.c`
- `original/examples/csvtest.c`
- `original/examples/csvvalid.c`
- `downstream/apps/csvutils-csvbreak.sh`
- `downstream/apps/csvutils-csvcheck.sh`
- `downstream/apps/csvutils-csvcount.sh`
- `downstream/apps/csvutils-csvcut.sh`
- `downstream/apps/csvutils-csvfix.sh`
- `downstream/apps/csvutils-csvgrep.sh`
- `downstream/apps/libcsv-example-csvfix.sh`
- `downstream/apps/libcsv-example-csvinfo.sh`
- `downstream/apps/libcsv-example-csvtest.sh`
- `downstream/apps/libcsv-example-csvvalid.sh`
- `downstream/apps/readstat.sh`
- `downstream/apps/tellico.sh`
- `downstream/fixtures/readstat/input.csv`
- `downstream/fixtures/readstat/metadata.json`
- `downstream/fixtures/shared/bad-malformed.csv`
- `downstream/fixtures/shared/bad-unterminated.csv`
- `downstream/fixtures/shared/quoted-users.csv`
- `safe/tests/c/abi_edges.c`
- `safe/tests/c/allocator_failures.c`
- `safe/tests/c/common.h`
- `safe/tests/c/layout_probe.c`
- `safe/tests/c/public_header_smoke.c`
- `safe/debian/changelog`
- `safe/debian/libcsv3.install`
- `safe/debian/watch`
- `safe/debian/upstream/metadata`
- `safe/debian/copyright`
- `safe/debian/libcsv-dev.docs`
- `safe/debian/libcsv-dev.examples`
- `safe/debian/libcsv-dev.install`
- `safe/debian/source/format`
