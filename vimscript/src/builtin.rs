use std::sync::Arc;

use crate::{VimScriptCtx, BuiltinFunction, value::{Value, Function, VimType}, VimError, State, Command, CmdRange};

struct Eval;

impl<S: State> BuiltinFunction<S> for Eval {
    fn execute(&self, args: Vec<Value>, ctx: &mut VimScriptCtx<S>, state: &mut S) -> Result<Value, VimError> {
        let expr: String = args.iter().map(|a| a.to_string(ctx)).collect();
        ctx.eval(expr.as_str(), state)
    }
}

struct Exec;

impl<S: State> BuiltinFunction<S> for Exec {
    fn execute(&self, args: Vec<Value>, ctx: &mut VimScriptCtx<S>, state: &mut S) -> Result<Value, VimError> {
        let expr: String = args.iter().map(|a| a.to_string(ctx)).collect();
        ctx.run(expr.as_str(), state).map(|_| Value::Nil)
    }
}

struct Builtin<F>(F);

impl<S, F: Fn(Vec<Value>, &mut VimScriptCtx<S>) -> Result<Value, VimError>> BuiltinFunction<S> for Builtin<F> {
    fn execute(&self, args: Vec<Value>, ctx: &mut VimScriptCtx<S>, _s: &mut S) -> Result<Value, VimError> {
        self.0(args, ctx)
    }
}

impl<E> Into<Result<Value, E>> for Value {
    fn into(self) -> Result<Value, E> {
        Ok(self)
    }
}

macro_rules! nargs {
    (|$ctx:ident $(,$param:ident $(= $default:ident)?)* $(,)?| $expr:expr) => {
        Function::Builtin(Arc::new(Builtin(|v: Vec<Value>, $ctx: &mut VimScriptCtx<_>| -> Result<Value, VimError> {
            const COUNT: usize = nargs!(@COUNT $(($param))*);
            let mut iter = v.into_iter();
            $(
                let $param = nargs!(@EXPAND iter.next() $(; $default)?, VimError::WrongArgCount(COUNT));
            )*
            if iter.next().is_some() {
                Err(VimError::WrongArgCount(COUNT))
            } else {
                $expr.into()
            }
        })))
    };
    (assert |$ctx:ident $(,$param:ident)*| $expr:expr) => {
        Function::Builtin(Arc::new(Builtin(|v: Vec<Value>, $ctx: &mut _| {
            let tmp: Result<&[Value; nargs!(@COUNT $(($param))*)], _> = v.as_slice().try_into();
            if let Ok([$($param,)*]) = tmp {
                if $expr {
                    Ok(Value::Nil)
                } else {
                    Err(VimError::Exit)
                }
            } else {
                Err(VimError::WrongArgCount(nargs!(@COUNT $(($param))*)))
            }
        })))
    };
    (@EXPAND $el:expr ; $default:expr, $err:expr) => {
        $el.unwrap_or($default)
    };
    (@EXPAND $el:expr, $err:expr) => {
        $el.ok_or($err)?
    };
    (@COUNT) => {
        0
    };
    (@COUNT $(($param:ident))*) => {
        [$(nargs!(@ONE $param),)*].len()
    };
    (@ONE $param:ident) => { () };
}

fn fmod(a: f64, b: f64) -> f64 {
    a - (a / b).trunc() * b
}

impl<S: State> VimScriptCtx<S> {
    pub fn builtin_functions(&mut self) {
        use Value::Nil;
// 	nr2char()		get a character by its number value
// 	list2str()		get a character string from a list of numbers
        self.functions.insert_builtin("char2nr", nargs!(|ctx, a| Value::Integer(a.to_string(ctx).chars().next().map_or(0, |c| c as isize))));
// 	char2nr()		get number value of a character
// 	str2list()		get list of numbers from a string
// 	str2nr()		convert a string to a Number
// 	str2float()		convert a string to a Float
// 	printf()		format a string according to % items
// 	escape()		escape characters in a string with a '\'
// 	shellescape()		escape a string for use with a shell command
// 	fnameescape()		escape a file name for use with a Vim command
// 	tr()			translate characters from one set to another
// 	strtrans()		translate a string to make it printable
        self.functions.insert_builtin("tolower", nargs!(|ctx, a| Value::Str(a.to_string(ctx).to_lowercase())));
// 	tolower()		turn a string to lowercase
        self.functions.insert_builtin("tolower", nargs!(|ctx, a| Value::Str(a.to_string(ctx).to_uppercase())));
// 	toupper()		turn a string to uppercase
// 	match()			position where a pattern matches in a string
// 	matchend()		position where a pattern match ends in a string
// 	matchfuzzy()		fuzzy matches a string in a list of strings
// 	matchfuzzypos()		fuzzy matches a string in a list of strings
// 	matchstr()		match of a pattern in a string
// 	matchstrpos()		match and positions of a pattern in a string
// 	matchlist()		like matchstr() and also return submatches
// 	stridx()		first index of a short string in a long string
// 	strridx()		last index of a short string in a long string
        self.functions.insert_builtin("strlen", nargs!(|ctx, a| Value::Integer(a.to_string(ctx).len() as isize)));
// 	strlen()		length of a string in bytes
        self.functions.insert_builtin("strlen", nargs!(|ctx, a| Value::Integer(a.to_string(ctx).chars().count() as isize)));
// 	strchars()		length of a string iProvidn characters
// 	strwidth()		size of string when displayed
// 	strdisplaywidth()	size of string when displayed, deals with tabs
// 	substitute()		substitute a pattern match with a string
// 	submatch()		get a specific match in ":s" and substitute()
// 	strpart()		get part of a string using byte index
// 	strcharpart()		get part of a string using char index
// 	strgetchar()		get character from a string using char index
// 	byteidx()		byte index of a character in a string
// 	byteidxcomp()		like byteidx() but count composing characters
// 	charidx()		character index of a byte in a string
        self.functions.insert_builtin("repeat", nargs!(|ctx, a, b| Value::Str(a.to_string(ctx).repeat(b.to_int(ctx)? as usize))));
// 	repeat()		repeat a string multiple times
        self.functions.insert_builtin("eval", Function::Builtin(Arc::new(Eval)));
// 	eval()			evaluate a string expression
        self.functions.insert_builtin("exec", Function::Builtin(Arc::new(Exec)));
// 	execute()		execute an Ex command and get the output
        self.functions.insert_builtin("trim", nargs!(|ctx, a| Value::Str(a.to_string(ctx).trim().to_string())));
// 	trim()			trim characters from a string
//
// List manipulation:					*list-functions*
        self.functions.insert_builtin("get", nargs!(|ctx, a, b| a.index(&b, ctx)));
// 	get()			get an item without error for wrong index
        self.functions.insert_builtin("len", nargs!(|_ctx, a| a.len()));
// 	len()			number of items in a List
        self.functions.insert_builtin("empty", nargs!(|_ctx, a| a.empty()));
// 	empty()			check if List is empty
        self.functions.insert_builtin("insert", nargs!(|ctx, a, b, c = Nil| a.insert(c.nil_or(|| Value::Integer(0))?, b, ctx)));
// 	insert()		insert an item somewhere in a List
        self.functions.insert_builtin("add", nargs!(|ctx, a, b| a.insert(a.len()?, b, ctx)));
// 	add()			append an item to a List
        self.functions.insert_builtin("extend", nargs!(|ctx, a, b, c = Nil| a.extend(b, c.nil_or(|| a.len())?, ctx)));
// 	extend()		append a List to a List
        self.functions.insert_builtin("remove", nargs!(|ctx, a, b| a.remove(b, ctx)));
// 	remove()		remove one or more items from a List
        self.functions.insert_builtin("copy", nargs!(|ctx, a| a));
// 	copy()			make a shallow copy of a List
        self.functions.insert_builtin("deepcopy", nargs!(|ctx, a| a.deep_copy()));
// 	deepcopy()		make a full copy of a List
        self.functions.insert_builtin("filter", nargs!(|ctx, a, b| a.filter(b, ctx)));
// 	filter()		remove selected items from a List
        self.functions.insert_builtin("map", nargs!(|ctx, a, b| a.map(b, ctx)));
// 	map()			change each List item
        self.functions.insert_builtin("sort", nargs!(|ctx, a, b = Nil, c = Nil| a.sort(b, c, ctx)));
// 	sort()			sort a List
        self.functions.insert_builtin("reverse", nargs!(|ctx, a| a.reverse(ctx)));
// 	reverse()		reverse the order of a List
        self.functions.insert_builtin("uniq", nargs!(|ctx, a, b = Nil, c = Nil| a.unique(b, c, ctx)));
// 	uniq()			remove copies of repeated adjacent items
        self.functions.insert_builtin("split", nargs!(|ctx, a, b = Nil, c = Nil| a.split(b, c, ctx)));
// 	split()			split a String into a List
        self.functions.insert_builtin("join", nargs!(|ctx, a, b = Nil| a.join(b, ctx)));
// 	join()			join List items into a String
        self.functions.insert_builtin("range", nargs!(|ctx, a, b = Nil, c = Nil| a.range(b, c, ctx)));
// 	range()			return a List with a sequence of numbers
        self.functions.insert_builtin("string", nargs!(|ctx, a| Value::Str(a.to_string(ctx))));
// 	string()		String representation of a List
        self.functions.insert_builtin("call", nargs!(|ctx, a, b, c = Nil| a.call(b, c, ctx)));
// 	call()			call a function with List as arguments
        // self.functions.insert_builtin("index", nargs!(|ctx, a, b, c = Nil| a.call(b, c, ctx)));
// 	index()			index of a value in a List
        self.functions.insert_builtin("max", nargs!(|ctx, a| a.max(ctx)));
// 	max()			maximum value in a List
        self.functions.insert_builtin("min", nargs!(|ctx, a| a.min(ctx)));
// 	min()			minimum value in a List
        self.functions.insert_builtin("count", nargs!(|ctx, a, b, c = Nil, d = Nil| a.count(b, c, d, ctx)));
// 	count()			count number of times a value appears in a List
        self.functions.insert_builtin("repeat", nargs!(|ctx, a, b| a.repeat(b, ctx)));
// 	repeat()		repeat a List multiple times
        self.functions.insert_builtin("flatten", nargs!(|ctx, a, b = Nil| a.flatten(b, ctx)));
// 	flatten()		flatten a List
//
// Dictionary manipulation:				*dict-functions*
        self.functions.insert_builtin("has_key", nargs!(|ctx, a, b| a.has_key(b, ctx)));
// 	has_key()		check whether a key appears in a Dictionary
        self.functions.insert_builtin("keys", nargs!(|ctx, a| a.keys(ctx)));
// 	keys()			get List of Dictionary keys
        self.functions.insert_builtin("values", nargs!(|ctx, a| a.values(ctx)));
// 	values()		get List of Dictionary values
        self.functions.insert_builtin("items", nargs!(|ctx, a| a.items(ctx)));
// 	items()			get List of Dictionary key-value pairs
//
// Floating point computation:				*float-functions*
        self.functions.insert_builtin("float2nr", nargs!(|ctx, a| Value::Integer(a.to_int(ctx)?)));
// 	float2nr()		convert Float to Number
        self.functions.insert_builtin("abs", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.abs())));
// 	abs()			absolute value (also works for Number)
        self.functions.insert_builtin("round", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.round())));
// 	round()			round off
        self.functions.insert_builtin("ceil", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.ceil())));
// 	ceil()			round up
        self.functions.insert_builtin("floor", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.floor())));
// 	floor()			round down
        self.functions.insert_builtin("trunc", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.trunc())));
// 	trunc()			remove value after decimal point
        self.functions.insert_builtin("fmod", nargs!(|ctx, a, b| Value::Number(fmod(a.to_num(ctx)?, b.to_num(ctx)?))));
// 	fmod()			remainder of division
        self.functions.insert_builtin("exp", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.exp())));
// 	exp()			exponential
        self.functions.insert_builtin("log", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.log(std::f64::consts::E))));
// 	log()			natural logarithm (logarithm to base e)
        self.functions.insert_builtin("log10", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.log10())));
// 	log10()			logarithm to base 10
        self.functions.insert_builtin("pow", nargs!(|ctx, a, b| Value::Number(a.to_num(ctx)?.powf(b.to_num(ctx)?))));
// 	pow()			value of x to the exponent y
        self.functions.insert_builtin("sqrt", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.sqrt())));
// 	sqrt()			square root
        self.functions.insert_builtin("sin", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.sin())));
// 	sin()			sine
        self.functions.insert_builtin("cos", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.cos())));
// 	cos()			cosine
        self.functions.insert_builtin("tan", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.tan())));
// 	tan()			tangent
        self.functions.insert_builtin("asin", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.asin())));
// 	asin()			arc sine
        self.functions.insert_builtin("acos", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.acos())));
// 	acos()			arc cosine
        self.functions.insert_builtin("atan", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.atan())));
// 	atan()			arc tangent
        self.functions.insert_builtin("atan2", nargs!(|ctx, a, b| Value::Number(a.to_num(ctx)?.atan2(b.to_num(ctx)?))));
// 	atan2()			arc tangent
        self.functions.insert_builtin("sinh", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.sinh())));
// 	sinh()			hyperbolic sine
        self.functions.insert_builtin("cosh", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.cosh())));
// 	cosh()			hyperbolic cosine
        self.functions.insert_builtin("tanh", nargs!(|ctx, a| Value::Number(a.to_num(ctx)?.tanh())));
// 	tanh()			hyperbolic tangent
//
// Other computation:					*bitwise-function*
        self.functions.insert_builtin("and", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx)? & b.to_int(ctx)?)));
// 	and()			bitwise AND
        self.functions.insert_builtin("invert", nargs!(|ctx, a| Value::Integer(!a.to_int(ctx)?)));
// 	invert()		bitwise invert
        self.functions.insert_builtin("or", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx)? | b.to_int(ctx)?)));
// 	or()			bitwise OR
        self.functions.insert_builtin("xor", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx)? ^ b.to_int(ctx)?)));
// 	xor()			bitwise XOR
        // self.functions.insert_builtin("sha256", nargs!(|ctx, a| todo!("sha256")));
// 	sha256()		SHA-256 hash
//
// Variables:						*var-functions*
// 	type()			type of a variable
        self.functions.insert_builtin("type", nargs!(|ctx, a| Value::Integer(a.ty().as_int())));
// 	islocked()		check if a variable is locked
// 	funcref()		get a Funcref for a function reference
// 	function()		get a Funcref for a function name
        self.functions.insert_builtin("garbagecollect", nargs!(|ctx| Value::Nil));
// 	garbagecollect()	possibly free memory
//
// Testing:				    *test-functions*
        self.functions.insert_builtin("assert_equal", nargs!(assert |_c, a, b| a == b));
// 	assert_equal()		assert that two expressions values are equal
// 	assert_equalfile()	assert that two file contents are equal
        self.functions.insert_builtin("assert_notequal", nargs!(assert |_c, a, b| a != b));
// 	assert_notequal()	assert that two expressions values are not equal
// 	assert_inrange()	assert that an expression is inside a range
// 	assert_match()		assert that a pattern matches the value
// 	assert_notmatch()	assert that a pattern does not match the value
        self.functions.insert_builtin("assert_false", nargs!(assert |ctx, a| !a.to_bool(ctx)?));
// 	assert_false()		assert that an expression is false
        self.functions.insert_builtin("assert_true", nargs!(assert |ctx, a| a.to_bool(ctx)?));
// 	assert_true()		assert that an expression is true
// 	assert_exception()	assert that a command throws an exception
// 	assert_beeps()		assert that a command beeps
// 	assert_nobeep()		assert that a command does not cause a beep
// 	assert_fails()		assert that a command fails
//
// Timers:						*timer-functions*
// 	timer_start()		create a timer
// 	timer_pause()		pause or unpause a timer
// 	timer_stop()		stop a timer
// 	timer_stopall()		stop all timers
// 	timer_info()		get information about timers
// 	wait()			wait for a condition
//
// Context Stack:					*ctx-functions*
// 	ctxget()		return context at given index from top
// 	ctxpop()		pop and restore top context
// 	ctxpush()		push given context
// 	ctxset()		set context at given index from top
// 	ctxsize()		return context stack size
//
// Various:					*various-functions*
        self.functions.insert_builtin("exists", nargs!(|ctx, a| Value::Bool(ctx.lookup(a.to_string(ctx)).is_ok())));
// 	exists()		check if a variable, function, etc. exists
//
// 	libcall()		call a function in an external library
// 	libcallnr()		idem, returning a number
//
// 	luaeval()		evaluate Lua expression
// 	py3eval()		evaluate Python expression (|+python3|)
// 	pyeval()		evaluate Python expression (|+python|)
// 	pyxeval()		evaluate |python_x| expression
// 	debugbreak()		interrupt a program being debugged

    }
}

struct Cmd<F>(F);

impl<S, F: Fn(CmdRange<'_>, bool, &str, &mut VimScriptCtx<S>, &mut S)> Command<S> for Cmd<F> {
    fn execute(
        &self,
        range: CmdRange<'_>,
        bang: bool,
        commands: &str,
        ctx: &mut VimScriptCtx<S>,
        state: &mut S,
    ) {
        self.0(range, bang, commands, ctx, state);
    }
}

macro_rules! cmd {
    (|$range:ident, $bang:ident, $args:ident, $ctx:ident, $state:ident| $expr:expr) => {
        {
            fn cmd_impl<S: State>($range: CmdRange<'_>, $bang: bool, $args: &str, $ctx: &mut VimScriptCtx<S>, $state: &mut S) {
                $expr;
            }
            Arc::new(Cmd(cmd_impl))
        }
    };
}

impl<S: State + 'static> VimScriptCtx<S> {
    pub fn builtin_commands(&mut self) {
        self.commands.insert("call".into(), cmd!(|_range, _bang, args, ctx, state| if let Err(e) = ctx.eval(args, state) {
            state.echo(format_args!("Error: {e:?}"));
        }));
        self.commands.insert("echo".into(), cmd!(|_range, _bang, args, ctx, state| match ctx.eval(args, state) {
            Ok(v) => state.echo(format_args!("{v}")),
            Err(e) => state.echo(format_args!("Error: {e:?}")),
        }));
    }
}
