use std::{ffi::OsStr, fmt::Display, fs, path::Path};

use code::Code;
use compiler::Compiler;
use env::{BitwiseMode, ContinueMode, LuaVersion, Options};
use parser::{parse_tokens, Expression};
use preprocessor::{preprocess_code, preprocess_codes, read_file};
use scanner::{scan_code, Token};

#[cfg(feature = "rpmalloc")]
#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

pub mod code;
pub mod compiler;
pub mod env;
pub mod parser;
pub mod preprocessor;
pub mod scanner;

#[macro_export]
macro_rules! check {
	($tocheck: expr) => {
		match $tocheck {
			Ok(t) => t,
			Err(e) => return Err(e.to_string()),
		}
	};
}

#[macro_export]
macro_rules! format_clue {
	($($strings:expr),+) => {{
		use std::ops::AddAssign;
		let mut len_format_clue = 0;
		$(len_format_clue.add_assign(AsRef::<str>::as_ref(&$strings).len());)+
		let mut output_format_clue = String::with_capacity(len_format_clue);
		$(output_format_clue.push_str($strings.as_ref());)+
		output_format_clue
	}};
}

pub struct Clue {
	options: Options,
}

impl Clue {
	pub fn new() -> Self {
		Clue {
			options: Options::default(),
		}
	}

	pub fn tokens(&mut self, env_tokens: bool) {
		self.options.env_tokens = env_tokens;
	}
	pub fn env_struct(&mut self, env_tokens: bool) {
		self.options.env_tokens = env_tokens;
	}

	pub fn bitwise_mode(&mut self, mode: BitwiseMode) {
		self.options.env_bitwise = mode;
	}

	pub fn continue_mode(&mut self, mode: ContinueMode) {
		self.options.env_continue = mode;
	}

	pub fn rawsetglobals(&mut self, env_rawsetglobal: bool) {
		self.options.env_rawsetglobals = env_rawsetglobal;
	}

	pub fn debug(&mut self, env_debug: bool) {
		self.options.env_debug = env_debug;
	}

	pub fn output(&mut self, output: bool) {
		self.options.env_output = output;
	}

	pub fn target(&mut self, version: Option<LuaVersion>) {
		self.options.env_target = version;
		self.options.preset();
	}

	pub fn target_os(&mut self, os: String) {
		self.options.env_targetos = os;
	}
}

impl Clue {
	pub fn preprocess_code(&self, code: String) -> Result<Code, String> {
		let mut code = code;
		let filename = String::from("(library)");
		let (codes, variables, ..) = preprocess_code(
			unsafe { code.as_bytes_mut() },
			1,
			false,
			&filename,
			&self.options,
		)?;
		preprocess_codes(0, codes, &variables, &filename)
	}
	pub fn preprocess_file<P: AsRef<Path> + AsRef<OsStr> + Display>(
		&self,
		path: P,
	) -> Result<Code, String> {
		let filepath: &Path = path.as_ref();
		let filename = filepath.file_name().unwrap().to_string_lossy().into_owned();
		let (codes, variables) = read_file(path, &filename, &self.options)?;
		preprocess_codes(0, codes, &variables, &filename)
	}
}

impl Clue {
	pub fn scan_preprocessed_file<P: AsRef<Path> + AsRef<OsStr> + Display>(
		&self,
		code: Code,
		path: P,
	) -> Result<Vec<Token>, String> {
		let filepath: &Path = path.as_ref();
		let filename = filepath.file_name().unwrap().to_string_lossy().into_owned();
		scan_code(code, &filename)
	}

	pub fn scan_preprocessed(&self, code: Code) -> Result<Vec<Token>, String> {
		scan_code(code, &String::from("(library)"))
	}

	pub fn scan_code(&self, code: String) -> Result<Vec<Token>, String> {
		let code = self.preprocess_code(code)?;
		self.scan_preprocessed(code)
	}
	pub fn scan_file<P: AsRef<Path> + AsRef<OsStr> + Display>(
		&self,
		filename: P,
	) -> Result<Vec<Token>, String> {
		let code = self.preprocess_file(&filename)?;
		self.scan_preprocessed_file(code, &filename)
	}
}

impl Clue {
	pub fn parse_preprocessed(&self, code: Code) -> Result<(Expression, String), String> {
		let tokens = self.scan_preprocessed(code)?;
		self.parse_tokens(tokens)
	}
	pub fn parse_tokens(&self, tokens: Vec<Token>) -> Result<(Expression, String), String> {
		parse_tokens(tokens, &String::from("(library)"), &self.options)
	}
	pub fn parse_code(&self, code: String) -> Result<(Expression, String), String> {
		let tokens = self.scan_code(code)?;
		self.parse_tokens(tokens)
	}
	pub fn parse_file<P: AsRef<Path> + AsRef<OsStr> + Display>(
		&self,
		path: P,
	) -> Result<(Expression, String), String> {
		let filepath: &Path = path.as_ref();
		let filename = filepath.file_name().unwrap().to_string_lossy().into_owned();
		let tokens = self.scan_file(&path)?;

		parse_tokens(tokens, &filename, &self.options)
	}
}

impl Clue {
	pub fn compile_tokens(&self, tokens: Vec<Token>) -> Result<String, String> {
		let (ctokens, statics) = self.parse_tokens(tokens)?;
		let compiler = Compiler::new(&self.options);
		Ok(statics + &compiler.compile_tokens(0, ctokens))
	}
	pub fn compile_preprocessed(&self, code: Code) -> Result<String, String> {
		let tokens = self.scan_preprocessed(code)?;
		self.compile_tokens(tokens)
	}
	pub fn compile_ast(&self, (ctokens, statics): (Expression, String)) -> Result<String, String> {
		let compiler = Compiler::new(&self.options);
		Ok(statics + &compiler.compile_tokens(0, ctokens))
	}
	pub fn compile_code(&self, code: String) -> Result<String, String> {
		let tokens = self.scan_code(code)?;
		self.compile_tokens(tokens)
	}
	pub fn compile_file<P: AsRef<Path> + AsRef<OsStr> + Display>(
		&self,
		path: P,
	) -> Result<String, String> {
		let tokens = self.scan_file(&path)?;
		let result = self.compile_tokens(tokens)?;
		if self.options.env_output {
			fs::write(path, &result).map_err(|e| e.to_string())?;
		}
		Ok(result)
	}
}

impl Default for Clue {
	fn default() -> Self {
		Clue::new()
	}
}
