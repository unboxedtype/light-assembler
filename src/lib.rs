/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

use std::{collections::HashMap, ops::RangeInclusive};
use ton_types::{Cell, SliceData, BuilderData};
use std::ffi::CString;
use std::os::raw::c_char;

pub use debug::{Line, Lines, DbgInfo, lines_to_string};

mod errors;
pub use errors::{
    CompileError, OperationError, ParameterError, Position,
    ToOperationParameterError,
};

mod debug;
mod macros;
mod parse;
mod complex;
mod simple;
mod convert;

mod writer;
use writer::{Units, Unit};
pub use debug::DbgPos;

// Basic types *****************************************************************
/// Operation Compilation result
type CompileResult = Result<(), OperationError>;
type CompileHandler = fn(&mut Engine, &[&str], destination: &mut Units, pos: DbgPos) -> CompileResult;

// CompileError::Operation handlers ***********************************************************
trait EnsureParametersCountInRange {
    fn assert_empty(&self) -> Result<(), OperationError>;
    fn assert_len(&self, _n: usize) -> Result<(), OperationError>;
    fn assert_len_in(&self, _r: RangeInclusive<usize>) -> Result<(), OperationError>;
}

impl<T> EnsureParametersCountInRange for [T] {
    fn assert_empty(&self) -> Result<(), OperationError> {
        self.assert_len_in(0..=0)
    }

    fn assert_len(&self, n: usize) -> Result<(), OperationError> {
        self.assert_len_in(n..=n)
    }

    fn assert_len_in(&self, range: RangeInclusive<usize>) -> Result<(), OperationError> {
        if &self.len() < range.start() {
            Err(OperationError::MissingRequiredParameters)
        } else if &self.len() > range.end() {
            Err(OperationError::TooManyParameters)
        } else {
            Ok(())
        }
    }
}

// Command compilation context ************************************************

struct CommandContext
{
    operation: String,
    line_no_cmd: usize,
    char_no_cmd: usize,
    line_no_par: usize,
    char_no_par: usize,
    rule_option: Option<CompileHandler>,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            operation: String::new(),
            line_no_cmd: 0,
            char_no_cmd: 0,
            line_no_par: 0,
            char_no_par: 0,
            rule_option: None,
        }
    }
}

impl CommandContext {
    fn new(operation: String, char_no_cmd: usize, line_no_cmd: usize, rule_option: Option<CompileHandler>) -> Self {
        Self {
            operation,
            line_no_cmd,
            char_no_cmd,
            line_no_par: 0,
            char_no_par: 0,
            rule_option,
        }
    }
    fn abort<X>(&self, error: OperationError, engine: &Engine) -> Result<X, CompileError> {
        if let Some(line) = engine.lines.get(self.line_no_cmd - 1) {
            let pos = &line.pos;
            let filename = pos.filename.clone();
            let line = pos.line_code;
            Err(CompileError::operation(line, self.char_no_cmd, self.operation.clone(), error).with_filename(filename))
        } else {
            Err(CompileError::operation(self.line_no_cmd, self.char_no_cmd, self.operation.clone(), error))
        }
    }
    fn has_command(&self) -> bool {
        self.rule_option.is_some()
    }
    fn compile(
        &mut self,
        destination: &mut Units,
        par: &mut Vec<Token>,
        engine: &mut Engine,
    ) -> Result<(), CompileError> {
        let rule = match self.rule_option.as_ref() {
            Some(rule) => rule,
            None => return Ok(())
        };
        let (line_no, char_no) = engine.set_pos(self.line_no_par, self.char_no_par);
        let mut n = par.len();
        loop {
            let par = par[0..n].iter().map(|p| p.token).collect::<Vec<_>>();
            let pos = if let Some(line) = engine.lines.get(self.line_no_cmd - 1) {
                line.pos.clone()
            } else {
                DbgPos::default()
            };
            match rule(engine, &par, destination, pos) {
                Ok(_) => break,
                Err(OperationError::TooManyParameters) if n != 0 => {
                    n -= 1;
                }
                Err(e) => return self.abort(e, engine)
            }
        }
        engine.set_pos(line_no, char_no);
        self.rule_option = None;
        // detecting some errors here
        if n > 1 && self.operation != "IFREFELSEREF" { // the only insn taking two blocks without comma between
            for token in &par[1..n] {
                if !token.was_comma {
                    if let Some(line) = engine.lines.get(token.line - 1) {
                        let pos = &line.pos;
                        return Err(CompileError::syntax(pos.line_code, token.column, "Missing comma").with_filename(pos.filename.clone()))
                    } else {
                        return Err(CompileError::syntax(token.line, token.column, "Missing comma"))
                    }
                }
            }
        }
        par.drain(..n);
        if !par.is_empty() {
            let token = par.remove(0);
            let position = if let Some(line) = engine.lines.get(token.line - 1) {
                let pos = &line.pos;
                let filename = pos.filename.clone();
                let line = pos.line_code;
                Position::new(filename, line, token.column)
            } else {
                Position::new(String::new(), token.line, token.column)
            };
            if token.was_comma || n == 0 {
                return Err(CompileError::Operation(
                    position,
                    self.operation.clone(),
                    OperationError::TooManyParameters,
                ))
            } else {
                // or CompileError::Syntax "missing comma"
                return Err(CompileError::UnknownOperation(
                    position, token.token.into()
                ))
            }
        }
        Ok(())
    }
}

// Compilation engine *********************************************************

#[allow(non_snake_case)]
pub struct Engine {
    line_no: usize,
    char_no: usize,
    lines: Lines,
    handlers: HashMap<&'static str, CompileHandler>,
    named_units: HashMap<String, Unit>,
}

struct Token<'a> {
    line: usize,
    column: usize,
    token: &'a str,
    was_comma: bool,
}

impl<'a> Token<'a> {
    fn new(line: usize, column: usize, token: &'a str, was_comma: bool) -> Self {
        Self { line, column, token, was_comma }
    }
}

impl Engine {
    pub fn new(lines: Lines) -> Self {
        let mut ret = Self {
            line_no: 1,
            char_no: 1,
            lines,
            handlers: HashMap::new(),
            named_units: HashMap::new(),
        };
        ret.add_complex_commands();
        ret.add_simple_commands();
        ret
    }

    fn is_whitespace(x: char) -> bool {
        matches!(x, ' ' | '\n' | '\r' | '\t')
    }

    fn set_pos(&mut self, line_no: usize, char_no: usize) -> (usize, usize) {
        let l = std::mem::replace(&mut self.line_no, line_no);
        let c = std::mem::replace(&mut self.char_no, char_no);
        (l, c)
    }

    pub fn build(&mut self, name: Option<String>, lines: Lines) -> Result<Unit, CompileError> {
        let source = lines_to_string(&lines);
        self.lines = lines;
        self.line_no = 1;
        self.char_no = 1;
        let (builder, dbg) = self.compile(&source)?.finalize();
        let unit = Unit::new(builder, dbg);
        if let Some(name) = name {
            self.named_units.insert(name, unit.clone());
        }
        Ok(unit)
    }

    fn compile(&mut self, source: &str) -> Result<Units, CompileError> {
        let mut ret = Units::new();
        let mut par = Vec::new();
        let mut acc = (0, 0);
        let mut expect_comma = false;
        let mut comma_found = false;
        let mut was_comma = false; // was comma before token
        let mut was_newline = false; // was line break before token
        let mut in_block = 0;
        let mut in_comment = false;
        let mut command_ctx = CommandContext::default();
        for ch in source.chars().chain(" ".chars()) {
            let mut newline_found = false;
            // Adjust line/char information
            let mut x = self.char_no;
            let y = self.line_no;
            if ch == '\n' {
                self.line_no += 1;
                self.char_no = 1
            } else {
                self.char_no += 1
            }
            let (s0, s1) = acc;
            let new_s1 = s1 + ch.len_utf8();
            // Process internal block if any
            if in_block > 0 {
                if ch == '{' {
                    in_block += 1
                } else if ch == '}' {
                    in_block -= 1
                }
                if in_block == 0 {
                    par.push(Token::new(y, x, &source[s0..s1], comma_found));
                    acc = (new_s1, new_s1)
                } else {
                    acc = (s0, new_s1)
                }
                continue;
            }
            // Process comment if any
            if in_comment {
                if (ch == '\r') || (ch == '\n') {
                    in_comment = false;
                    was_newline = true;
                }
                acc = (new_s1, new_s1);
                continue;
            }
            // Analyze char
            if Engine::is_whitespace(ch) {
                if (ch == '\r') || (ch == '\n') {
                    newline_found = true;
                    was_newline = true;
                }
                acc = (new_s1, new_s1);
                if s0 == s1 {
                    continue;
                }
            } else if ch == ';' {
                acc = (new_s1, new_s1);
                in_comment = true;
                if s0 == s1 {
                    continue;
                }
            } else if ch == ',' {
                if !expect_comma {
                    if let Some(line) = self.lines.get(y - 1) {
                        let pos = &line.pos;
                        return Err(CompileError::syntax(pos.line_code, x, ",").with_filename(pos.filename.clone()))
                    } else {
                        return Err(CompileError::syntax(y, x, ","))
                    }
                }
                acc = (new_s1, new_s1);
                expect_comma = false;
                comma_found = true;
                if s0 == s1 {
                    continue;
                }
            } else if ch == '{' {
                if expect_comma || !command_ctx.has_command() {
                    if let Some(line) = self.lines.get(y - 1) {
                        let pos = &line.pos;
                        return Err(CompileError::syntax(pos.line_code, x, ch).with_filename(pos.filename.clone()))
                    } else {
                        return Err(CompileError::syntax(y, x, ch))
                    }
                }
                acc = (new_s1, new_s1);
                in_block = 1;
                command_ctx.line_no_par = self.line_no;
                command_ctx.char_no_par = self.char_no;
                continue;
            } else if ch == '}' {
                if let Some(line) = self.lines.get(y - 1) {
                    let pos = &line.pos;
                    return Err(CompileError::syntax(pos.line_code, x, ch).with_filename(pos.filename.clone()))
                } else {
                    return Err(CompileError::syntax(y, x, ch))
                }
            } else if ch.is_ascii_alphanumeric() || (ch == '-') || (ch == '_') || (ch == '.') {
                acc = (s0, new_s1);
                if s0 == s1 { //start of new token
                    was_comma = comma_found;
                    comma_found = false;
                    expect_comma = true
                }
                continue;
            } else { // TODO: (message for the owner: please write descriptive explanation)
                if let Some(line) = self.lines.get(y - 1) {
                    let pos = &line.pos;
                    return Err(CompileError::syntax(pos.line_code, x, "Bad char").with_filename(pos.filename.clone()))
                } else {
                    return Err(CompileError::syntax(y, x, "Bad char"))
                }
            }
            // Token extracted
            let token = source[s0..s1].to_ascii_uppercase();
            log::trace!(target: "tvm", "--> {}\n", token);
            x -= token.chars().count();
            match self.handlers.get(token.as_str()) {
                None => {
                    if command_ctx.has_command() {
                        par.push(Token::new(y, x, &source[s0..s1], was_comma));
                        was_comma = false;
                        continue
                    } else if let Some(line) = self.lines.get(y - 1) {
                        let pos = &line.pos;
                        return Err(CompileError::unknown(pos.line_code, x, &token).with_filename(pos.filename.clone()))
                    } else {
                        return Err(CompileError::unknown(y, x, &token))
                    }
                }
                Some(&new_rule) => {
                    match command_ctx.compile(&mut ret, &mut par, self) {
                        Ok(_) => {
                            command_ctx = CommandContext::new(token, x, y, Some(new_rule));
                            expect_comma = false;
                            was_comma = false;
                            was_newline = newline_found;
                        }
                        Err(e @ CompileError::Operation(_, _, OperationError::MissingRequiredParameters)) => {
                            if was_newline { // it seems realy new command - rturn correct missing params error
                                return Err(e)
                            } else {
                                par.push(Token::new(y, x, &source[s0..s1], was_comma));
                                was_comma = false;
                            }
                        }
                        Err(e) => return Err(e)
                    }
                }
            }
        }
        // Compile last pending command if any
        command_ctx.compile(&mut ret, &mut par, self)?;
        if in_block != 0 {
            if let Some(line) = self.lines.get(self.line_no - 1) {
                let pos = &line.pos;
                return Err(CompileError::syntax(pos.line_code, 0, "Missing }").with_filename(pos.filename.clone()))
            } else {
                return Err(CompileError::syntax(self.line_no, 0, "Missing }"))
            }
        }
        Ok(ret)
    }

}

pub fn compile_code_to_builder(code: &str) -> Result<BuilderData, CompileError> {
    log::trace!(target: "tvm", "begin compile\n");
    Ok(Engine::new(vec![]).compile(code)?.finalize().0)
}

pub fn compile_code(code: &str) -> Result<SliceData, CompileError> {
    let code = compile_code_to_builder(code)?;
    match SliceData::load_builder(code) {
        Ok(code) => Ok(code),
        Err(_) => Err(CompileError::unknown(0, 0, "failure while convert BuilderData to cell"))
    }
}

pub fn compile_code_to_cell(code: &str) -> Result<Cell, CompileError> {
    log::trace!(target: "tvm", "begin compile\n");
    let code = compile_code_to_builder(code)?;
    match code.into_cell() {
        Ok(code) => Ok(code),
        Err(_) => Err(CompileError::unknown(0, 0, "failure while convert BuilderData to cell"))
    }
}

#[no_mangle]
pub extern "C" fn compile_code_to_b64(ptr: *mut c_char) -> *mut c_char {
    unsafe {
        if !ptr.is_null() {
            let cstr = CString::from_raw(ptr) ;
            let str = cstr.to_str().unwrap();
            let code = compile_code_to_builder(&str).unwrap();
            let result =
                CString::new(
                    match code.into_cell() {
                        Ok(code) =>
                        { base64::encode(ton_types::boc::write_boc(&code).unwrap()) }
                        Err(_) =>
                            "error".into()
                    }
                );
            result.unwrap().into_raw()
        } else {
            panic!("null pointer passed");
        }
    }
}

pub fn compile_code_debuggable(code: Lines) -> Result<(SliceData, DbgInfo), CompileError> {
    log::trace!(target: "tvm", "begin compile\n");
    let source = lines_to_string(&code);
    let (builder, dbg) = Engine::new(code).compile(&source)?.finalize();
    let cell = builder.into_cell().unwrap();
    match SliceData::load_cell(cell.clone()) {
        Ok(code) => {
            let dbg_info = DbgInfo::from(cell, dbg);
            Ok((code, dbg_info))
        }
        Err(_) => Err(CompileError::unknown(0, 0, "failure while convert BuilderData to cell"))
    }
}
