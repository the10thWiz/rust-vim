use std::sync::Arc;

use crate::{VimScriptCtx, BuiltinFunction, value::{Value, Function}, VimError, State, Command, CmdRange};

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

macro_rules! nargs {
    (|$ctx:ident, $($param:ident),*| $expr:expr) => {
        Function::Builtin(Arc::new(Builtin(|v: Vec<Value>, $ctx: &mut _| {
            let tmp: Result<&[Value; nargs!(@COUNT $($param)*)], _> = v.as_slice().try_into();
            if let Ok([$($param,)*]) = tmp {
                Ok($expr)
            } else {
                Err(VimError::WrongArgCount)
            }
        })))
    };
    (assert |$ctx:ident, $($param:ident),*| $expr:expr) => {
        Function::Builtin(Arc::new(Builtin(|v: Vec<Value>, $ctx: &mut _| {
            let tmp: Result<&[Value; nargs!(@COUNT $($param)*)], _> = v.as_slice().try_into();
            if let Ok([$($param,)*]) = tmp {
                if $expr {
                    Ok(Value::Nil)
                } else {
                    Err(VimError::Exit)
                }
            } else {
                Err(VimError::WrongArgCount)
            }
        })))
    };
    (@COUNT $($param:ident)*) => {
        [$(nargs!(@ONE $param), )*].len()
    };
    (@ONE $param:ident) => {
        ()
    };
}

impl<S: State> VimScriptCtx<S> {
    pub fn builtin_functions(&mut self) {
        self.functions.insert_builtin("nr2char", nargs!(|ctx, a| Value::Integer(a.to_string(ctx).chars().next().map_or(0, |c| c as isize))));
// 	nr2char()		get a character by its number value
// 	list2str()		get a character string from a list of numbers
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
// 	strchars()		length of a string in characters
// 	strwidth()		size of string when displayed
// 	strdisplaywidth()	size of string when displayed, deals with tabs
// 	substitute()		substitute a pattern match with a string
// 	submatch()		get a specific match in ":s" and substitute()
// 	strpart()		get part of a string using byte index
// 	strcharpart()		get part of a string using char index
// 	strgetchar()		get character from a string using char index
// 	expand()		expand special keywords
// 	expandcmd()		expand a command like done for `:edit`
// 	iconv()			convert text from one encoding to another
// 	byteidx()		byte index of a character in a string
// 	byteidxcomp()		like byteidx() but count composing characters
// 	charidx()		character index of a byte in a string
        self.functions.insert_builtin("repeat", nargs!(|ctx, a, b| Value::Str(a.to_string(ctx).repeat(b.to_int(ctx) as usize))));
// 	repeat()		repeat a string multiple times
        self.functions.insert_builtin("eval", Function::Builtin(Arc::new(Eval)));
// 	eval()			evaluate a string expression
        self.functions.insert_builtin("exec", Function::Builtin(Arc::new(Exec)));
// 	execute()		execute an Ex command and get the output
// 	win_execute()		like execute() but in a specified window
        self.functions.insert_builtin("trim", nargs!(|ctx, a| Value::Str(a.to_string(ctx).trim().to_string())));
// 	trim()			trim characters from a string
//
// List manipulation:					*list-functions*
// 	get()			get an item without error for wrong index
// 	len()			number of items in a List
// 	empty()			check if List is empty
// 	insert()		insert an item somewhere in a List
// 	add()			append an item to a List
// 	extend()		append a List to a List
// 	remove()		remove one or more items from a List
// 	copy()			make a shallow copy of a List
// 	deepcopy()		make a full copy of a List
// 	filter()		remove selected items from a List
// 	map()			change each List item
// 	sort()			sort a List
// 	reverse()		reverse the order of a List
// 	uniq()			remove copies of repeated adjacent items
// 	split()			split a String into a List
// 	join()			join List items into a String
// 	range()			return a List with a sequence of numbers
// 	string()		String representation of a List
// 	call()			call a function with List as arguments
// 	index()			index of a value in a List
// 	max()			maximum value in a List
// 	min()			minimum value in a List
// 	count()			count number of times a value appears in a List
// 	repeat()		repeat a List multiple times
// 	flatten()		flatten a List
//
// Dictionary manipulation:				*dict-functions*
// 	get()			get an entry without an error for a wrong key
// 	len()			number of entries in a Dictionary
// 	has_key()		check whether a key appears in a Dictionary
// 	empty()			check if Dictionary is empty
// 	remove()		remove an entry from a Dictionary
// 	extend()		add entries from one Dictionary to another
// 	filter()		remove selected entries from a Dictionary
// 	map()			change each Dictionary entry
// 	keys()			get List of Dictionary keys
// 	values()		get List of Dictionary values
// 	items()			get List of Dictionary key-value pairs
// 	copy()			make a shallow copy of a Dictionary
// 	deepcopy()		make a full copy of a Dictionary
// 	string()		String representation of a Dictionary
// 	max()			maximum value in a Dictionary
// 	min()			minimum value in a Dictionary
        // self.functions.insert_builtin("count", nargs!(|ctx, a| Value::Integer(a.to_int(ctx))));
// 	count()			count number of times a value appears
//
// Floating point computation:				*float-functions*
        self.functions.insert_builtin("float2nr", nargs!(|ctx, a| Value::Integer(a.to_int(ctx))));
// 	float2nr()		convert Float to Number
        self.functions.insert_builtin("abs", nargs!(|ctx, a| Value::Number(a.to_num(ctx).abs())));
// 	abs()			absolute value (also works for Number)
        self.functions.insert_builtin("round", nargs!(|ctx, a| Value::Number(a.to_num(ctx).round())));
// 	round()			round off
        self.functions.insert_builtin("ceil", nargs!(|ctx, a| Value::Number(a.to_num(ctx).ceil())));
// 	ceil()			round up
        self.functions.insert_builtin("floor", nargs!(|ctx, a| Value::Number(a.to_num(ctx).floor())));
// 	floor()			round down
        self.functions.insert_builtin("trunc", nargs!(|ctx, a| Value::Number(a.to_num(ctx).trunc())));
// 	trunc()			remove value after decimal point
// 	fmod()			remainder of division
        self.functions.insert_builtin("exp", nargs!(|ctx, a| Value::Number(a.to_num(ctx).exp())));
// 	exp()			exponential
        self.functions.insert_builtin("log", nargs!(|ctx, a| Value::Number(a.to_num(ctx).log(std::f64::consts::E))));
// 	log()			natural logarithm (logarithm to base e)
        self.functions.insert_builtin("log10", nargs!(|ctx, a| Value::Number(a.to_num(ctx).log10())));
// 	log10()			logarithm to base 10
        self.functions.insert_builtin("pow", nargs!(|ctx, a, b| Value::Number(a.to_num(ctx).powf(b.to_num(ctx)))));
// 	pow()			value of x to the exponent y
        self.functions.insert_builtin("sqrt", nargs!(|ctx, a| Value::Number(a.to_num(ctx).sqrt())));
// 	sqrt()			square root
        self.functions.insert_builtin("sin", nargs!(|ctx, a| Value::Number(a.to_num(ctx).sin())));
// 	sin()			sine
        self.functions.insert_builtin("cos", nargs!(|ctx, a| Value::Number(a.to_num(ctx).cos())));
// 	cos()			cosine
        self.functions.insert_builtin("tan", nargs!(|ctx, a| Value::Number(a.to_num(ctx).tan())));
// 	tan()			tangent
        self.functions.insert_builtin("asin", nargs!(|ctx, a| Value::Number(a.to_num(ctx).asin())));
// 	asin()			arc sine
        self.functions.insert_builtin("acos", nargs!(|ctx, a| Value::Number(a.to_num(ctx).acos())));
// 	acos()			arc cosine
        self.functions.insert_builtin("atan", nargs!(|ctx, a| Value::Number(a.to_num(ctx).atan())));
// 	atan()			arc tangent
        self.functions.insert_builtin("atan2", nargs!(|ctx, a, b| Value::Number(a.to_num(ctx).atan2(b.to_num(ctx)))));
// 	atan2()			arc tangent
        self.functions.insert_builtin("sinh", nargs!(|ctx, a| Value::Number(a.to_num(ctx).sinh())));
// 	sinh()			hyperbolic sine
        self.functions.insert_builtin("cosh", nargs!(|ctx, a| Value::Number(a.to_num(ctx).cosh())));
// 	cosh()			hyperbolic cosine
        self.functions.insert_builtin("tanh", nargs!(|ctx, a| Value::Number(a.to_num(ctx).tanh())));
// 	tanh()			hyperbolic tangent
//
// Other computation:					*bitwise-function*
        self.functions.insert_builtin("and", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx) & b.to_int(ctx))));
// 	and()			bitwise AND
        self.functions.insert_builtin("invert", nargs!(|ctx, a| Value::Integer(!a.to_int(ctx))));
// 	invert()		bitwise invert
        self.functions.insert_builtin("or", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx) | b.to_int(ctx))));
// 	or()			bitwise OR
        self.functions.insert_builtin("xor", nargs!(|ctx, a, b| Value::Integer(a.to_int(ctx) ^ b.to_int(ctx))));
// 	xor()			bitwise XOR
// 	sha256()		SHA-256 hash
//
// Variables:						*var-functions*
// 	type()			type of a variable
// 	islocked()		check if a variable is locked
// 	funcref()		get a Funcref for a function reference
// 	function()		get a Funcref for a function name
// 	getbufvar()		get a variable value from a specific buffer
// 	setbufvar()		set a variable in a specific buffer
// 	getwinvar()		get a variable from specific window
// 	gettabvar()		get a variable from specific tab page
// 	gettabwinvar()		get a variable from specific window & tab page
// 	setwinvar()		set a variable in a specific window
// 	settabvar()		set a variable in a specific tab page
// 	settabwinvar()		set a variable in a specific window & tab page
// 	garbagecollect()	possibly free memory
//
// Cursor and mark position:		*cursor-functions* *mark-functions*
// 	col()			column number of the cursor or a mark
// 	virtcol()		screen column of the cursor or a mark
// 	line()			line number of the cursor or mark
// 	wincol()		window column number of the cursor
// 	winline()		window line number of the cursor
// 	cursor()		position the cursor at a line/column
// 	screencol()		get screen column of the cursor
// 	screenrow()		get screen row of the cursor
// 	getcurpos()		get position of the cursor
// 	getpos()		get position of cursor, mark, etc.
// 	setpos()		set position of cursor, mark, etc.
// 	getmarklist()		list of global/local marks
// 	byte2line()		get line number at a specific byte count
// 	line2byte()		byte count at a specific line
// 	diff_filler()		get the number of filler lines above a line
// 	screenattr()		get attribute at a screen line/row
// 	screenchar()		get character code at a screen line/row
// 	screenchars()		get character codes at a screen line/row
// 	screenstring()		get string of characters at a screen line/row
// 	charcol()		character number of the cursor or a mark
// 	getcharpos()		get character position of cursor, mark, etc.
// 	setcharpos()		set character position of cursor, mark, etc.
// 	getcursorcharpos()	get character position of the cursor
// 	setcursorcharpos()	set character position of the cursor
//
// Working with text in the current buffer:		*text-functions*
// 	getline()		get a line or list of lines from the buffer
// 	setline()		replace a line in the buffer
// 	append()		append line or list of lines in the buffer
// 	indent()		indent of a specific line
// 	cindent()		indent according to C indenting
// 	lispindent()		indent according to Lisp indenting
// 	nextnonblank()		find next non-blank line
// 	prevnonblank()		find previous non-blank line
// 	search()		find a match for a pattern
// 	searchpos()		find a match for a pattern
// 	searchpair()		find the other end of a start/skip/end
// 	searchpairpos()		find the other end of a start/skip/end
// 	searchdecl()		search for the declaration of a name
// 	getcharsearch()		return character search information
// 	setcharsearch()		set character search information
//
// 					*system-functions* *file-functions*
// System functions and manipulation of files:
// 	glob()			expand wildcards
// 	globpath()		expand wildcards in a number of directories
// 	glob2regpat()		convert a glob pattern into a search pattern
// 	findfile()		find a file in a list of directories
// 	finddir()		find a directory in a list of directories
// 	resolve()		find out where a shortcut points to
// 	fnamemodify()		modify a file name
// 	pathshorten()		shorten directory names in a path
// 	simplify()		simplify a path without changing its meaning
// 	executable()		check if an executable program exists
// 	exepath()		full path of an executable program
// 	filereadable()		check if a file can be read
// 	filewritable()		check if a file can be written to
// 	getfperm()		get the permissions of a file
// 	setfperm()		set the permissions of a file
// 	getftype()		get the kind of a file
// 	isdirectory()		check if a directory exists
// 	getfsize()		get the size of a file
// 	getcwd()		get the current working directory
// 	haslocaldir()		check if current window used |:lcd| or |:tcd|
// 	tempname()		get the name of a temporary file
// 	mkdir()			create a new directory
// 	chdir()			change current working directory
// 	delete()		delete a file
// 	rename()		rename a file
// 	system()		get the result of a shell command as a string
// 	systemlist()		get the result of a shell command as a list
// 	environ()		get all environment variables
// 	getenv()		get one environment variable
// 	setenv()		set an environment variable
// 	hostname()		name of the system
// 	readfile()		read a file into a List of lines
// 	readdir()		get a List of file names in a directory
// 	writefile()		write a List of lines or Blob into a file
//
// Date and Time:				*date-functions* *time-functions*
// 	getftime()		get last modification time of a file
// 	localtime()		get current time in seconds
// 	strftime()		convert time to a string
// 	strptime()		convert a date/time string to time
// 	reltime()		get the current or elapsed time accurately
// 	reltimestr()		convert reltime() result to a string
// 	reltimefloat()		convert reltime() result to a Float
//
// 			*buffer-functions* *window-functions* *arg-functions*
// Buffers, windows and the argument list:
// 	argc()			number of entries in the argument list
// 	argidx()		current position in the argument list
// 	arglistid()		get id of the argument list
// 	argv()			get one entry from the argument list
// 	bufexists()		check if a buffer exists
// 	buflisted()		check if a buffer exists and is listed
// 	bufloaded()		check if a buffer exists and is loaded
// 	bufname()		get the name of a specific buffer
// 	bufnr()			get the buffer number of a specific buffer
// 	tabpagebuflist()	return List of buffers in a tab page
// 	tabpagenr()		get the number of a tab page
// 	tabpagewinnr()		like winnr() for a specified tab page
// 	winnr()			get the window number for the current window
// 	bufwinid()		get the window ID of a specific buffer
// 	bufwinnr()		get the window number of a specific buffer
// 	winbufnr()		get the buffer number of a specific window
// 	getbufline()		get a list of lines from the specified buffer
// 	setbufline()		replace a line in the specified buffer
// 	appendbufline()		append a list of lines in the specified buffer
// 	deletebufline()		delete lines from a specified buffer
// 	win_findbuf()		find windows containing a buffer
// 	win_getid()		get window ID of a window
// 	win_gotoid()		go to window with ID
// 	win_id2tabwin()		get tab and window nr from window ID
// 	win_id2win()		get window nr from window ID
// 	win_move_separator()	move window vertical separator
// 	win_move_statusline()	move window status line
// 	getbufinfo()		get a list with buffer information
// 	gettabinfo()		get a list with tab page information
// 	getwininfo()		get a list with window information
// 	getchangelist()		get a list of change list entries
// 	getjumplist()		get a list of jump list entries
// 	swapinfo()		information about a swap file
// 	swapname()		get the swap file path of a buffer
//
// Command line:					*command-line-functions*
// 	getcmdline()		get the current command line
// 	getcmdpos()		get position of the cursor in the command line
// 	setcmdpos()		set position of the cursor in the command line
// 	getcmdtype()		return the current command-line type
// 	getcmdwintype()		return the current command-line window type
// 	getcompletion()		list of command-line completion matches
// 	fullcommand()		get full command name
//
// Quickfix and location lists:			*quickfix-functions*
// 	getqflist()		list of quickfix errors
// 	setqflist()		modify a quickfix list
// 	getloclist()		list of location list items
// 	setloclist()		modify a location list
//
// Insert mode completion:				*completion-functions*
// 	complete()		set found matches
// 	complete_add()		add to found matches
// 	complete_check()	check if completion should be aborted
// 	complete_info()		get current completion information
// 	pumvisible()		check if the popup menu is displayed
// 	pum_getpos()		position and size of popup menu if visible
//
// Folding:					*folding-functions*
// 	foldclosed()		check for a closed fold at a specific line
// 	foldclosedend()		like foldclosed() but return the last line
// 	foldlevel()		check for the fold level at a specific line
// 	foldtext()		generate the line displayed for a closed fold
// 	foldtextresult()	get the text displayed for a closed fold
//
// Syntax and highlighting:	  *syntax-functions* *highlighting-functions*
// 	clearmatches()		clear all matches defined by |matchadd()| and
// 	getmatches()		get all matches defined by |matchadd()| and
// 	hlexists()		check if a highlight group exists
// 	hlID()			get ID of a highlight group
// 	synID()			get syntax ID at a specific position
// 	synIDattr()		get a specific attribute of a syntax ID
// 	synIDtrans()		get translated syntax ID
// 	synstack()		get list of syntax IDs at a specific position
// 	synconcealed()		get info about concealing
// 	diff_hlID()		get highlight ID for diff mode at a position
// 	matchadd()		define a pattern to highlight (a "match")
// 	matchaddpos()		define a list of positions to highlight
// 	matcharg()		get info about |:match| arguments
// 	matchdelete()		delete a match defined by |matchadd()| or a
// 	setmatches()		restore a list of matches saved by
//
// Spelling:					*spell-functions*
// 	spellbadword()		locate badly spelled word at or after cursor
// 	spellsuggest()		return suggested spelling corrections
// 	soundfold()		return the sound-a-like equivalent of a word
//
// History:					*history-functions*
// 	histadd()		add an item to a history
// 	histdel()		delete an item from a history
// 	histget()		get an item from a history
// 	histnr()		get highest index of a history list
//
// Interactive:					*interactive-functions*
// 	browse()		put up a file requester
// 	browsedir()		put up a directory requester
// 	confirm()		let the user make a choice
// 	getchar()		get a character from the user
// 	getcharmod()		get modifiers for the last typed character
// 	feedkeys()		put characters in the typeahead queue
// 	input()			get a line from the user
// 	inputlist()		let the user pick an entry from a list
// 	inputsecret()		get a line from the user without showing it
// 	inputdialog()		get a line from the user in a dialog
// 	inputsave()		save and clear typeahead
// 	inputrestore()		restore typeahead
//
// GUI:						*gui-functions*
// 	getfontname()		get name of current font being used
// 	getwinpos()		position of the Vim window
// 	getwinposx()		X position of the Vim window
// 	getwinposy()		Y position of the Vim window
// 	balloon_show()		set the balloon content
// 	balloon_split()		split a message for a balloon
// 	balloon_gettext()	get the text in the balloon
//
// Vim server:					*server-functions*
// 	serverlist()		return the list of server names
// 	remote_startserver()	run a server
// 	remote_send()		send command characters to a Vim server
// 	remote_expr()		evaluate an expression in a Vim server
// 	server2client()		send a reply to a client of a Vim server
// 	remote_peek()		check if there is a reply from a Vim server
// 	remote_read()		read a reply from a Vim server
// 	foreground()		move the Vim window to the foreground
// 	remote_foreground()	move the Vim server window to the foreground
//
// Window size and position:			*window-size-functions*
// 	winheight()		get height of a specific window
// 	winwidth()		get width of a specific window
// 	win_screenpos()		get screen position of a window
// 	winlayout()		get layout of windows in a tab page
// 	winrestcmd()		return command to restore window sizes
// 	winsaveview()		get view of current window
// 	winrestview()		restore saved view of current window
//
// Mappings:				    *mapping-functions*
// 	digraph_get()		get |digraph|
// 	digraph_getlist()	get all |digraph|s
// 	digraph_set()		register |digraph|
// 	digraph_setlist()	register multiple |digraph|s
// 	hasmapto()		check if a mapping exists
// 	mapcheck()		check if a matching mapping exists
// 	maparg()		get rhs of a mapping
// 	wildmenumode()		check if the wildmode is active
//
// Signs:						*sign-functions*
// 	sign_define()		define or update a sign
// 	sign_getdefined()	get a list of defined signs
// 	sign_getplaced()	get a list of placed signs
// 	sign_jump()		jump to a sign
// 	sign_place()		place a sign
// 	sign_placelist()	place a list of signs
// 	sign_undefine()		undefine a sign
// 	sign_unplace()		unplace a sign
// 	sign_unplacelist()	unplace a list of signs
//
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
        self.functions.insert_builtin("assert_false", nargs!(assert |ctx, a| !a.to_bool(ctx)));
// 	assert_false()		assert that an expression is false
        self.functions.insert_builtin("assert_true", nargs!(assert |ctx, a| a.to_bool(ctx)));
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
// Tags:						*tag-functions*
// 	taglist()		get list of matching tags
// 	tagfiles()		get a list of tags files
// 	gettagstack()		get the tag stack of a window
// 	settagstack()		modify the tag stack of a window
//
// Prompt Buffer:					*promptbuffer-functions*
// 	prompt_getprompt()	get the effective prompt text for a buffer
// 	prompt_setcallback()	set prompt callback for a buffer
// 	prompt_setinterrupt()	set interrupt callback for a buffer
// 	prompt_setprompt()	set the prompt text for a buffer
//
// Context Stack:					*ctx-functions*
// 	ctxget()		return context at given index from top
// 	ctxpop()		pop and restore top context
// 	ctxpush()		push given context
// 	ctxset()		set context at given index from top
// 	ctxsize()		return context stack size
//
// Various:					*various-functions*
// 	mode()			get current editing mode
// 	visualmode()		last visual mode used
// 	exists()		check if a variable, function, etc. exists
// 	has()			check if a feature is supported in Vim
// 	changenr()		return number of most recent change
// 	cscope_connection()	check if a cscope connection exists
// 	did_filetype()		check if a FileType autocommand was used
// 	eventhandler()		check if invoked by an event handler
// 	getpid()		get process ID of Vim
//
// 	libcall()		call a function in an external library
// 	libcallnr()		idem, returning a number
//
// 	undofile()		get the name of the undo file
// 	undotree()		return the state of the undo tree
//
// 	getreg()		get contents of a register
// 	getreginfo()		get information about a register
// 	getregtype()		get type of a register
// 	setreg()		set contents and type of a register
// 	reg_executing()		return the name of the register being executed
// 	reg_recording()		return the name of the register being recorded
//
// 	shiftwidth()		effective value of 'shiftwidth'
//
// 	wordcount()		get byte/word/char count of buffer
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
