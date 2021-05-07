use std::{error::Error as StdError, fmt::Display, path::PathBuf};
use std::convert::TryFrom;
use std::sync::Mutex;

use anyhow;
use bevy::prelude::info;
use ruruby::{self, *};
use lazy_static::lazy_static;

lazy_static! {
    // Output of script.
    pub static ref SCRIPT_COMMANDS: Mutex<Vec<ScriptCommand>> = {
        Mutex::new(Vec::new())
    };
}

pub enum ScriptCommand {
    SetVisible(String, bool),
}

#[derive(Debug, Clone)]
pub struct ScriptVm {
    pub vm: ruruby::VMRef,
    pub vm_context: ruruby::ContextRef,
    pub parser: ruruby::Parser,
}

impl ScriptVm {
    pub fn new() -> ScriptVm {
        let vm = new_interpreter();

        ScriptVm { vm,
                   vm_context: context(vm),
                   parser: ruruby::Parser::new() }
    }

    pub fn eval_repl_code(&mut self, code: &str) -> anyhow::Result<Value> {
        let parse_result = self.parser
                               .clone()
                               .parse_program_repl(PathBuf::from("dialogue"),
                                                   code.as_ref(),
                                                   Some(self.vm_context))
                               .map_err(|e| RubyStdError { source: e })?;
        let value = self.vm
                        .run_repl(parse_result, self.vm_context)
                        .map_err(|e| {
                            print_error(&e);
                            RubyStdError { source: e }
                        })?;

        Ok(value)
    }

    pub fn eval_repl_code_logging_result(&mut self, code: &str) {
        match self.eval_repl_code(code) {
            Ok(value) => {
                info!("result: {:?}", value);
            },
            Err(error) => {
                // Could be a parse error or a Ruby error.
                info!("error: {:?}", error);
            },
        }
    }

    pub fn require_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let abs_path = path.canonicalize()?;
        let program = self.vm
                          .load_file(&abs_path)
                          .map_err(|e| RubyStdError { source: e })?;
        self.vm.exec_program(abs_path, program.as_ref());
        Ok(())
    }
}

// Wrap RubyError so we can implement std::error::Error.
#[derive(Debug)]
pub struct RubyStdError {
    source: RubyError,
}

impl StdError for RubyStdError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

impl Display for RubyStdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RubyStdError: {:?}", self.source)
    }
}

fn print_error(err: &RubyError) {
    for (info, loc) in &err.info {
        info.show_loc(loc);
    }
    err.show_err();
}

pub fn new_interpreter() -> VMRef {
    let mut globals = GlobalsRef::new_globals();
    let vm = globals.create_main_fiber();
    let vals: Vec<Value> = vec!["script.rb"].iter()
                                            .map(|x| Value::string(*x))
                                            .collect();
    let argv = Value::array_from(vals);
    globals.set_toplevel_constant("ARGV", argv);

    // Patch with custom functions.
    let class = Module::class_under_object();
    // BuiltinClass::set_toplevel_constant() is private.
    BuiltinClass::object().set_const_by_str("ScriptCore", class.into());
    class.add_builtin_class_method("say", say);
    class.add_builtin_class_method("show_map_objects_by_name", show_map_objects_by_name);

    vm
}

pub fn context(vm: VMRef) -> ContextRef {
    ContextRef::new_heap(vm.globals.main_object,
                         Block::None,
                         ISeqRef::default(),
                         None)
}

fn show_map_objects_by_name(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let map_id = args[0].expect_integer("1st arg")?;
    let _map_id = u64::try_from(map_id).expect("map id overflowed range");
    // TODO: Constrain to the given map.
    let mut arg1 = args[1];
    let name = arg1.expect_string("2nd arg")?;
    let mut commands = SCRIPT_COMMANDS.lock().expect("mutex poisoned");
    commands.push(ScriptCommand::SetVisible(name.to_string(), true));
    Ok(Value::nil())
}

fn say(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;

    eprintln!("Hello, from Rust.  Called with: {:?}", args);

    match &args.block {
        Block::None => {
            // No block argument.
        }
        block => {
            let mut args = Args::new(1);
            let arg = Value::string("A string constructed in Rust.");
            args[0] = arg;
            vm.eval_block(block, &args)?;
        }
    }

    if args.len() >= 1 {
        // Extract first arg.
        let mut arg0 = args[0];
        let s = arg0.expect_string("first argument")?;
        // Print it.
        eprintln!("Message: {}", s);
    }

    if args.len() >= 2 {
        // Extract second arg.
        let num = args[1].expect_integer("second argument")?;
        // Return incremented.
        return Ok(Value::integer(num + 1));
    }

    Ok(Value::nil())
}
