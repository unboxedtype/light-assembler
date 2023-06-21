# Light-assembler

The Everscale Assembler distributed in a form of shared library. It is used in Light compiler, to natively compile TVM mnemonics into the bytecode.

## Prerequisites

* Rust
  https://www.rust-lang.org/en-US/install.html

## Build 

```console
git clone https://github.com/unboxedtype/light-assembler
cd light-assembler
make build
make install
```

This will compile the `libever_assembler.so` shared library and copy it into
the `/usr/lib64` directory.

If you ever need to remove it, execute:

```console
make uninstall
```
---
Copyright (C) 2019-2021 TON Labs. All Rights Reserved.

Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
this file except in compliance with the License.

You may obtain a copy of the
License at: https://www.ton.dev/licenses

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific TON DEV software governing permissions and
limitations under the License.
