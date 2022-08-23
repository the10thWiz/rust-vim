use std::{convert::TryInto, sync::Arc};

use vimscript::{BuiltinFunction, Value, VimError, VimScriptCtx};

use crate::VimInner;

struct Builtin<F>(F);

impl<S, F: Fn(Vec<Value>, &mut VimScriptCtx<S>, &mut S) -> Result<Value, VimError>>
    BuiltinFunction<S> for Builtin<F>
{
    fn execute(
        &self,
        args: Vec<Value>,
        ctx: &mut VimScriptCtx<S>,
        s: &mut S,
    ) -> Result<Value, VimError> {
        self.0(args, ctx, s)
    }
}

macro_rules! nargs {
    (|$ctx:ident, $state:ident $(,$param:ident)*| $expr:expr) => {
        Arc::new(Builtin(|v: Vec<Value>, $ctx: &mut VimScriptCtx<VimInner>, $state: &mut VimInner| {
            let tmp: Result<&[Value; nargs!(@COUNT $($param)*)], _> = v.as_slice().try_into();
            if let Ok([$($param,)*]) = tmp {
                Ok($expr)
            } else {
                Err(VimError::WrongArgCount(nargs!(@COUNT $($param)*)))
            }
        }))
    };
    (@COUNT) => {
        0
    };
    (@COUNT $($param:ident)*) => {
        [$(nargs!(@ONE $param), )*].len()
    };
    (@ONE $param:ident) => {
        ()
    };
}

pub fn builtin_functions(ctx: &mut VimScriptCtx<VimInner>) {
    // Cursor and mark position:		*cursor-functions* *mark-functions*
    ctx.builtin(
        "col",
        nargs!(|ctx, state, a| if a == "." {
            Value::Integer(state.get_focus().cursor().x + 1)
        } else if a == "$" {
            let win = state.get_focus();
            Value::Integer(win.buffer().read().get_line(win.cursor().y).unwrap().len() as isize + 1)
        } else if a.starts_with('\'') {
            todo!("Marks")
        } else if a == "v" {
            // TODO: visual selection
            Value::Integer(state.get_focus().cursor().x + 1)
        } else {
            Value::Integer(state.get_focus().cursor().x + 1)
        }),
    );
    // 	col()			column number of the cursor or a mark
    // 	virtcol()		screen column of the cursor or a mark
    ctx.builtin(
        "line",
        nargs!(|ctx, state, a| if a == "." {
            Value::Integer(state.get_focus().cursor().y + 1)
        } else if a == "$" {
            Value::Integer(state.get_focus().buffer().read().len())
        } else if a.starts_with('\'') {
            todo!("Marks")
        } else if a == "v" {
            // TODO: visual selection
            Value::Integer(state.get_focus().cursor().y + 1)
        } else {
            Value::Integer(state.get_focus().cursor().y + 1)
        }),
    );
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
    // Various:
    // 	mode()			get current editing mode
    // 	visualmode()		last visual mode used
    // 	has()			check if a feature is supported in Vim
    // 	changenr()		return number of most recent change
    // 	cscope_connection()	check if a cscope connection exists
    // 	did_filetype()		check if a FileType autocommand was used
    // 	eventhandler()		check if invoked by an event handler
    // 	getpid()		get process ID of Vim
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
    // 	expand()		expand special keywords
    // 	expandcmd()		expand a command like done for `:edit`
    // 	iconv()			convert text from one encoding to another
    // 	win_execute()		like execute() but in a specified window
    // 	getbufvar()		get a variable value from a specific buffer
    // 	setbufvar()		set a variable in a specific buffer
    // 	getwinvar()		get a variable from specific window
    // 	gettabvar()		get a variable from specific tab page
    // 	gettabwinvar()		get a variable from specific window & tab page
    // 	setwinvar()		set a variable in a specific window
    // 	settabvar()		set a variable in a specific tab page
    // 	settabwinvar()		set a variable in a specific window & tab page
}
