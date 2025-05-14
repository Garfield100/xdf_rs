# Fuzzing the XDF crate

This folder contains two small crates to easily fuzz the XDF crate with AFL and Honggfuzz: afl_xdf, and hfuzz_xdf.
These use [hfuzz](https://docs.rs/honggfuzz/latest/honggfuzz/) and [afl](https://docs.rs/afl/latest/afl/) crates, respectively.
Please use these crates' documentations (or just the AFL docs in that case) for more details on how for example find the crash files.
Each of these crates further contains a `Justfile` to make the fuzzing process an easy one-liner if you have [Just](https://github.com/casey/just) installed. You can list available recipes with `just -l` or `just --list`.

## Corpus

A fuzzing corpus is a set of input files on which a fuzzer will base its initial inputs. This can help greatly speed up the initial stages of fuzzing. Place some XDF files in the `corpus` folder. Each fuzzer will look for files there automatically.

## Honggfuzz

Honggfuzz (abbreviated Hfuzz) supports easy multithreading. If you have Just installed, you can start fuzzing with

```bash
cd hfuzz_xdf
just fuzz_hfuzz
```

You can stop the fuzzing process at any time with Ctrl+C. During startup this will create `hfuzz_target` and (for some reason two) `hfuzz_workspace` folders.

With `just fuzz_hfuzz <threads>` you can specify the number of threads Honggfuzz should use. The default is 10.

## AFL

AFL is another popular fuzzer. Multithreading with it is not as easy and requires some extra steps.
Start the main thread with

```bash
cd afl_xdf
just fuzz_afl
```

and optionally start additional threads in a new terminal with

```bash
just fuzz_afl <thread_id>
```

where `<thread_id>` should be a positive integer.
