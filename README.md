# Light-assembler

The Everscale Assembler distributed in a form of shared library. It is used in Light compiler, to natively compile TVM mnemonics into the bytecode.

## Prerequisites

* Rust
  https://www.rust-lang.org/en-US/install.html

## Build 

``git clone https://github.com/unboxedtype/light-assembler``
``cd light-assembler``
``make build``

This will compile the libever_assembler.so shared library.

``make install``

This will copy the libever_assembler.so lib into /usr/lib64, so the Compiler
can interact with it.

``make uninstall``

This will remove the .so file from /usr/lib64, when you no longer need it.

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
