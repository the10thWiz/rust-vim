use std::convert::TryFrom;

use bitfield::bitfield;
use vimscript::{Value, VimError};

macro_rules! str_enum {
    (enum $name:ident { $($var:ident $( = $alt:literal)?),* $(,)?}) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
        pub enum $name {
            $($var,)*
        }

        impl TryFrom<&str> for $name {
            type Error = VimError;
            fn try_from(v: &str) -> Result<Self, Self::Error> {
                match v {
                    $(stringify!($var) $(| $alt)? => Ok(Self::$var),)*
                    _ => Err(VimError::InvalidValue),
                }
            }
        }

        impl Into<&'static str> for $name {
            fn into(self) -> &'static str {
                match self {
                    $(Self::$var => stringify!($var),)*
                }
            }
        }
    };
    (struct $name:ident { $($var:ident $( = $alt:literal)?),* $(,)?}) => {
        bitfield! {
            #[derive(Clone, Copy, Hash, PartialEq, Eq)]
            pub struct $name(u64);
            impl Debug;
            $(pub $var, concat_idents!(set_, $var): 0;)*
        }

        impl TryFrom<&str> for $name {
            type Error = VimError;
            fn try_from(v: &str) -> Result<Self, Self::Error> {
                let mut ret = Self(0);
                for name in v.split(',') {
                    match name {
                        $(stringify!($var) => ret. concat_idents!(set_, $var)(true),)*
                        _ => Err(VimError::InvalidValue),
                    }
                }
                Ok(ret)
            }
        }
    };
}


str_enum!(
    enum BufHidden {
        global = "",
        hide,
        unload,
        delete,
        wipe,
    }
);
// TODO
str_enum!(
    enum BellOff {
        all,
        backspace,
        cursor,
        complete,
        copy,
        ctrlg,
        error,
        esc,
        hangul,
        insertmode,
        lang,
        mess,
        showmatch,
        operator,
        register,
        shell,
        spell,
        wildmode,
    }
);

macro_rules! options {
    ($opts:ident {$($name1:ident $(| $name2:ident $(| $name3:ident)?)? : $ty:ty => $val:expr),* $(,)?}) => {
        #[derive(Debug, Clone)]
        pub struct $opts {
            $(pub $name1: $ty,)*
        }

        impl $opts {
            pub fn new() -> Self {
                Self {
                    $($name1: $val.into(),)*
                }
            }

            pub fn get(&self, name: &str) -> Result<Value, VimError> {
                match name {
                    $(stringify!($name1) $(| stringify!($name2) $(| stringify!($name3))?)?  => Ok(Value::from(&self.$name1)),)*
                    _ => Err(VimError::VariableUndefined)
                }
            }

            pub fn set(&self, name: &str, val: &str) -> Result<(), VimError> {
                match name {
                    $(stringify!($name1) $(| stringify!($name2) $(| stringify!($name3))?)?  => todo!(),)*
                    _ => return Err(VimError::VariableUndefined),
                }
                Ok(())
            }

            pub fn set_commands() {
                todo!("Option commands")
            }
        }

        impl Default for $opts {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

options! {
    Options {
        aleph | al : isize => 224isize, // ASCII code of the letter Aleph (Hebrew)
        allowrevins | ari : bool => false, // allow CTRL-_ in Insert and Command-line mode
        ambiwidth | ambw : String => "single", // what to do with Unicode chars of ambiguous width
        autochdir | acd : bool => false, // change directory to the file in the current window
        arabicshape | arshape : bool => true, // do shaping for Arabic characters
        autoread | ar : bool => true, // autom. read file when changed outside of Vim
        autowrite | aw : bool => false, // automatically write file if changed
        autowriteall | awa : bool => false, // as 'autowrite', but works with more commands
        background | bg : String => "dark", // "dark" or "light", used for highlight colors
        backspace | bs : String => "indent,eol,start,nostop", // how backspace works at start of line
        backup | bk : bool => false, // keep backup file after overwriting a file
        backupcopy | bkc : String => "auto", // make backup as a copy, don't rename the file
        backupdir | bdir : String => ".,/home/matthew/.local/share/nvim/backup//", // list of directories for the backup file
        backupext | bex : String => "~", // extension used for the backup file
        backupskip | bsk : String => "/tmp/*", // no backup for files that match these patterns
        bomb : bool => false, // prepend a Byte Order Mark to the file
        breakat | brk : String => "     !@*-+;:,./?", // characters that may cause a line break
        browsedir | bsdir : String => "last", // which directory to start browsing in
        casemap | cmp : String => "internal,keepascii", // specifies how case of letters is changed
        cdhome | cdh : bool => false, // change directory to the home directory by ":cd"
        cdpath | cd : String => ",,", // list of directories searched with ":cd"
        cedit : String => "", // key used to open the command-line window
        charconvert | ccv : String => "", // expression for character encoding conversion
        clipboard | cb : String => "unnamedplus", // use the clipboard as the unnamed register
        cmdheight | ch : isize => 1isize, // number of lines to use for the command-line
        cmdwinheight | cwh : isize => 7isize, // height of the command-line window
        columns | co : isize => 80isize, // number of columns in the display
        completeopt | cot : String => "menuone,noselect", // options for Insert mode completion
        confirm | cf : bool => false, // ask what to do about unsaved/read-only files
        cpoptions | cpo : String => "aABceFs_", // flags for Vi-compatible behavior
        cscopepathcomp | cspc : isize => 0isize, // how many components of the path to show
        cscopeprg | csprg : String => "cscope", // command to execute cscope
        cscopequickfix | csqf : String => "", // use quickfix window for cscope results
        cscoperelative | csre : bool => false, // Use cscope.out path basename as prefix
        cscopetag | cst : bool => true, // use cscope for tag commands
        cscopetagorder | csto : isize => 0isize, // determines ":cstag" search order
        debug : String => "", // set to "msg" to see all error messages
        define | def : String => "^\\s*#\\s*define", // pattern to be used to find a macro definition
        delcombine | deco : bool => false, // delete combining characters on their own
        dictionary | dict : String => "", // list of file names used for keyword completion
        diffexpr | dex : String => "", // expression used to obtain a diff file
        diffopt | dip : String => "internal,filler,closeoff", // options for using diff mode
        digraph | dg : bool => false, // enable the entering of digraphs in Insert mode
        directory | dir : String => "/home/matthew/.local/share/nvim/swap//", // list of directory names for the swap file
        display | dy : String => "lastline,msgsep", // list of flags for how to display text
        eadirection | ead : String => "both", // in which direction 'equalalways' works
        emoji | emo : bool => true,
        encoding | enc : String => "UTF-8", // encoding used internally
        equalalways | ea : bool => true, // windows are automatically made the same size
        equalprg | ep : String => "", // external program to use for "=" command
        errorbells | eb : bool => false, // ring the bell for error messages
        errorfile | ef : String => "errors.err", // name of the errorfile for the QuickFix mode
        errorformat | efm : String => "%*[^\"]\"%f\"%*\\D%l: %m,\"%f\"%*\\D%l: %m,%-G%f:%l: (Each undeclared identifier is reported only once,%-G%f:%l: for each function it appears in.),%-GIn file included from %f:%l:%c:,%-GIn file included from %f:%l:%c\\,,%-GIn file included from %f:%l:%c,%-GIn file included from %f:%l,%-G%*[ ]from %f:%l:%c,%-G%*[ ]from %f:%l:,%-G%*[ ]from %f:%l\\,,%-G%*[ ]from %f:%l,%f:%l:%c:%m,%f(%l):%m,%f:%l:%m,\"%f\"\\, line %l%*\\D%c%*[^ ] %m,%D%*\\a[%*\\d]: Entering directory %*[`']%f',%X%*\\a[%*\\d]: Leaving directory %*[`']%f',%D%*\\a: Entering directory %*[`']%f',%X%*\\a: Leaving directory %*[`']%f',%DMaking %*\\a in %f,%f|%l| %m", // description of the lines in the error file
        eventignore | ei : String => "", // autocommand events that are ignored
        fileencodings | fencs : String => "ucs-bom,utf-8isize,default,latin1", // automatically detected character encodings
        fileignorecase | fic : bool => false, // ignore case when using file names
        fillchars | fcs : String => "", // characters to use for displaying special items
        foldclose | fcl : String => "", // close a fold when the cursor leaves it
        foldlevelstart | fdls : isize => -1isize, // when starting to edit a file
        formatexpr | fex : String => "", // expression used with "gq" command
        formatprg | fp : String => "", // name of external program used with "gq" command
        fsync | fs : bool => false, // whether to invoke fsync() after file write
        gdefault | gd : bool => false, // the ":substitute" flag 'g' is default on
        grepformat | gfm : String => "%f:%l:%m,%f:%l%m,%f  %l%m", // format of 'grepprg' output
        grepprg | gp : String => "grep -n ", // program to use for ":grep"
        guicursor | gcr : String => "n-v-c-sm:block,i-ci-ve:ver25isize,r-cr-o:hor20", // GUI: settings for cursor shape and blinking
        guifont | gfn : String => "", // GUI: Name(s) of font(s) to be used
        guifontwide | gfw : String => "", // list of font names for double-wide characters
        // guioptions | go : String => "", // GUI: Which components and options are used
        guitablabel | gtl : String => "", // GUI: custom label for a tab page
        guitabtooltip | gtt : isize => 0isize, // GUI: custom tooltip for a tab page
        helpfile | hf : isize => 0isize, // full path name of the main help file
        helpheight | hh : isize => 0isize, // minimum height of a new help window
        helplang | hlg : isize => 0isize, // preferred help languages
        hidden | hid : isize => 0isize, // don't unload buffer when it is |abandon|ed
        hlsearch | hls : isize => 0isize, // highlight matches with last search pattern
        history | hi : isize => 0isize, // number of command-lines that are remembered
        hkmap | hk : isize => 0isize, // Hebrew keyboard mapping
        hkmapp | hkp : isize => 0isize, // phonetic Hebrew keyboard mapping
        icon : isize => 0isize, // let Vim set the text of the window icon
        iconstring : isize => 0isize, // string to use for the Vim icon text
        ignorecase | ic : isize => 0isize, // ignore case in search patterns
        imcmdline | imc : isize => 0isize, // use IM when starting to edit a command line
        imdisable | imd : isize => 0isize, // do not use the IM in any mode
        iminsert | imi : isize => 0isize, // use :lmap or IM in Insert mode
        imsearch | ims : isize => 0isize, // use :lmap or IM when typing a search pattern
        include | inc : isize => 0isize, // pattern to be used to find an include file
        includeexpr | inex : isize => 0isize, // expression used to process an include line
        incsearch | is : isize => 0isize, // highlight match while typing search pattern
        indentexpr | inde : isize => 0isize, // expression used to obtain the indent of a line
        indentkeys | indk : isize => 0isize, // keys that trigger indenting with 'indentexpr'
        infercase | inf : isize => 0isize, // adjust case of match for keyword completion
        insertmode | im : isize => 0isize, // start the edit of a file in Insert mode
        isfname | isf : isize => 0isize, // characters included in file names and pathnames
        isident | isi : isize => 0isize, // characters included in identifiers
        iskeyword | isk : isize => 0isize, // characters included in keywords
        isprint | isp : isize => 0isize, // printable characters
        joinspaces | js : isize => 0isize, // two spaces after a period with a join command
        jumpoptions | jop : isize => 0isize, // specifies how jumping is done
        keymap | kmp : isize => 0isize, // name of a keyboard mapping
        keymodel | km : isize => 0isize, // enable starting/stopping selection with keys
        keywordprg | kp : isize => 0isize, // program to use for the "K" command
        langmap | lmap : isize => 0isize, // alphabetic characters for other language mode
        langmenu | lm : isize => 0isize, // language to be used for the menus
        langremap | lrm : isize => 0isize, // do apply 'langmap' to mapped characters
        laststatus | ls : isize => 0isize, // tells when last window has status lines
        lazyredraw | lz : isize => 0isize, // don't redraw while executing macros
        linebreak | lbr : isize => 0isize, // wrap long lines at a blank
        lines : isize => 0isize, // number of lines in the display
        linespace | lsp : isize => 0isize, // number of pixel lines to use between characters
        lisp : isize => 0isize, // automatic indenting for Lisp
        lispwords | lw : isize => 0isize, // words that change how lisp indenting works
        list : isize => 0isize, // show <Tab> and <EOL>
        listchars | lcs : isize => 0isize, // characters for displaying in list mode
        loadplugins | lpl : isize => 0isize, // load plugin scripts when starting up
        magic : isize => 0isize, // changes special characters in search patterns
        makeef | mef : isize => 0isize, // name of the errorfile for ":make"
        makeencoding | menc : isize => 0isize, // encoding of external make/grep commands
        makeprg | mp : isize => 0isize, // program to use for the ":make" command
        matchpairs | mps : isize => 0isize, // pairs of characters that "%" can match
        matchtime | mat : isize => 0isize, // tenths of a second to show matching paren
        maxcombine | mco : isize => 0isize, // maximum nr of combining characters displayed
        maxfuncdepth | mfd : isize => 0isize, // maximum recursive depth for user functions
        maxmapdepth | mmd : isize => 0isize, // maximum recursive depth for mapping
        maxmempattern | mmp : isize => 0isize, // maximum memory (in Kbyte) used for pattern search
        menuitems | mis : isize => 0isize, // maximum number of items in a menu
        mkspellmem | msm : isize => 0isize, // memory used before |:mkspell| compresses the tree
        modeline | ml : isize => 0isize, // recognize modelines at start or end of file
        modelineexpr | mle : isize => 0isize, // allow setting expression options from a modeline
        modelines | mls : isize => 0isize, // number of lines checked for modelines
        modifiable | ma : isize => 0isize, // changes to the text are not possible
        modified | mod : isize => 0isize, // buffer has been modified
        more : isize => 0isize, // pause listings when the whole screen is filled
        mouse : isize => 0isize, // enable the use of mouse clicks
        mousefocus | mousef : isize => 0isize, // keyboard focus follows the mouse
        mousehide | mh : isize => 0isize, // hide mouse pointer while typing
        mousemodel | mousem : isize => 0isize, // changes meaning of mouse buttons
        mouseshape | mouses : isize => 0isize, // shape of the mouse pointer in different modes
        mousetime | mouset : isize => 0isize, // max time between mouse double-click
        nrformats | nf : isize => 0isize, // number formats recognized for CTRL-A command
        number | nu : isize => 0isize, // print the line number in front of each line
        numberwidth | nuw : isize => 0isize, // number of columns used for the line number
        omnifunc | ofu : isize => 0isize, // function for filetype-specific completion
        opendevice | odev : isize => 0isize, // allow reading/writing devices on MS-Windows
        operatorfunc | opfunc : isize => 0isize, // function to be called for |g@| operator
        packpath | pp : isize => 0isize, // list of directories used for packages
        paragraphs | para : isize => 0isize, // nroff macros that separate paragraphs
        paste : isize => 0isize, // allow pasting text
        pastetoggle | pt : isize => 0isize, // key code that causes 'paste' to toggle
        patchexpr | pex : isize => 0isize, // expression used to patch a file
        patchmode | pm : isize => 0isize, // keep the oldest version of a file
        path | pa : isize => 0isize, // list of directories searched with "gf" et.al.
        perldll : isize => 0isize, // name of the Perl dynamic library
        preserveindent | pi : isize => 0isize, // preserve the indent structure when reindenting
        previewheight | pvh : isize => 0isize, // height of the preview window
        previewpopup | pvp : isize => 0isize, // use popup window for preview
        previewwindow | pvw : isize => 0isize, // identifies the preview window
        printdevice | pdev : isize => 0isize, // name of the printer to be used for :hardcopy
        printencoding | penc : isize => 0isize, // encoding to be used for printing
        printexpr | pexpr : isize => 0isize, // expression used to print PostScript for :hardcopy
        printfont | pfn : isize => 0isize, // name of the font to be used for :hardcopy
        printheader | pheader : isize => 0isize, // format of the header used for :hardcopy
        printmbcharset | pmbcs : isize => 0isize, // CJK character set to be used for :hardcopy
        printmbfont | pmbfn : isize => 0isize, // font names to be used for CJK output of :hardcopy
        printoptions | popt : isize => 0isize, // controls the format of :hardcopy output
        pumheight | ph : isize => 0isize, // maximum height of the popup menu
        pumwidth | pw : isize => 0isize, // minimum width of the popup menu
        pythondll : isize => 0isize, // name of the Python 2 dynamic library
        pythonthreedll : isize => 0isize, // name of the Python 3 dynamic library
        pyxversion | pyx : isize => 0isize, // Python version used for pyx* commands
        quoteescape | qe : isize => 0isize, // escape characters used in a string
        readonly | ro : isize => 0isize, // disallow writing the buffer
        redrawtime | rdt : isize => 0isize, // timeout for 'hlsearch' and |:match| highlighting
        regexpengine | re : isize => 0isize, // default regexp engine to use
        relativenumber | rnu : isize => 0isize, // show relative line number in front of each line
        remap : isize => 0isize, // allow mappings to work recursively
        report : isize => 0isize, // threshold for reporting nr. of lines changed
        revins | ri : isize => 0isize, // inserting characters will work backwards
        rightleft | rl : isize => 0isize, // window is right-to-left oriented
        rightleftcmd | rlc : isize => 0isize, // commands for which editing works right-to-left
        rubydll : isize => 0isize, // name of the Ruby dynamic library
        ruler | ru : bool => false, // show cursor line and column in the status line
        rulerformat | ruf : isize => 0isize, // custom format for the ruler
        runtimepath | rtp : String => "$XDG_CONFIG_HOME/rvim/", // list of directories used for runtime files
        scroll | scr : isize => 1isize, // lines to scroll with CTRL-U and CTRL-D
        scrollbind | scb : isize => 0isize, // scroll in window as other windows scroll
        scrolljump | sj : isize => 0isize, // minimum number of lines to scroll
        scrolloff | so : isize => 0isize, // minimum nr. of lines above and below cursor
        scrollopt | sbo : isize => 0isize, // how 'scrollbind' should behave
        sections | sect : isize => 0isize, // nroff macros that separate sections
        secure : isize => 0isize, // secure mode for reading .vimrc in current dir
        selection | sel : isize => 0isize, // what type of selection to use
        selectmode | slm : isize => 0isize, // when to use Select mode instead of Visual mode
        sessionoptions | ssop : isize => 0isize, // options for |:mksession|
        shada | sd : isize => 0isize, // use .shada file upon startup and exiting
        shell | sh : isize => 0isize, // name of shell to use for external commands
        shellcmdflag | shcf : isize => 0isize, // flag to shell to execute one command
        shellpipe | sp : isize => 0isize, // string to put output of ":make" in error file
        shellquote | shq : isize => 0isize, // quote character(s) for around shell command
        shellredir | srr : isize => 0isize, // string to put output of filter in a temp file
        shellslash | ssl : isize => 0isize, // use forward slash for shell file names
        shelltemp | stmp : isize => 0isize, // whether to use a temp file for shell commands
        shellxescape | sxe : isize => 0isize, // characters to escape when 'shellxquote' is (
        shellxquote | sxq : isize => 0isize, // like 'shellquote', but include redirection
        shiftround | sr : isize => 0isize, // round indent to multiple of shiftwidth
        shiftwidth | sw : isize => 0isize, // number of spaces to use for (auto)indent step
        shortmess | shm : isize => 0isize, // list of flags, reduce length of messages
        showbreak | sbr : isize => 0isize, // string to use at the start of wrapped lines
        showcmd | sc : isize => 0isize, // show (partial) command in status line
        showfulltag | sft : isize => 0isize, // show full tag pattern when completing tag
        showmatch | sm : isize => 0isize, // briefly jump to matching bracket if insert one
        showmode | smd : isize => 0isize, // message on status line to show current mode
        showtabline | stal : isize => 0isize, // tells when the tab pages line is displayed
        sidescroll | ss : isize => 0isize, // minimum number of columns to scroll horizontal
        sidescrolloff | siso : isize => 0isize, // min. nr. of columns to left and right of cursor
        signcolumn | scl : isize => 0isize, // when and how to display the sign column
        smartcase | scs : isize => 0isize, // no ignore case when pattern has uppercase
        smartindent | si : isize => 0isize, // smart autoindenting for C programs
        smarttab | sta : isize => 0isize, // use 'shiftwidth' when inserting <Tab>
        softtabstop | sts : isize => 0isize, // number of spaces that <Tab> uses while editing
        spell : isize => 0isize, // enable spell checking
        spellcapcheck | spc : isize => 0isize, // pattern to locate end of a sentence
        spellfile | spf : isize => 0isize, // files where |zg| and |zw| store words
        spelllang | spl : isize => 0isize, // language(s) to do spell checking for
        spelloptions | spo : isize => 0isize, // options for spell checking
        spellsuggest | sps : isize => 0isize, // method(s) used to suggest spelling corrections
        splitbelow | sb : isize => 0isize, // new window from split is below the current one
        splitright | spr : isize => 0isize, // new window is put right of the current one
        startofline | sol : isize => 0isize, // commands move cursor to first non-blank in line
        statusline | stl : isize => 0isize, // custom format for the status line
        suffixes | su : isize => 0isize, // suffixes that are ignored with multiple match
        suffixesadd | sua : isize => 0isize, // suffixes added when searching for a file
        swapfile | swf : isize => 0isize, // whether to use a swapfile for a buffer
        switchbuf | swb : isize => 0isize, // sets behavior when switching to another buffer
        synmaxcol | smc : isize => 0isize, // maximum column to find syntax items
        syntax | syn : isize => 0isize, // syntax to be loaded for current buffer
        tabline | tal : isize => 0isize, // custom format for the console tab pages line
        tabpagemax | tpm : isize => 0isize, // maximum number of tab pages for |-p| and "tab all"
        tabstop | ts : isize => 0isize, // number of spaces that <Tab> in file uses
        tagbsearch | tbs : isize => 0isize, // use binary searching in tags files
        tagcase | tc : isize => 0isize, // how to handle case when searching in tags files
        taglength | tl : isize => 0isize, // number of significant characters for a tag
        tagrelative | tr : isize => 0isize, // file names in tag file are relative
        tags | tag : isize => 0isize, // list of file names used by the tag command
        tagstack | tgst : isize => 0isize, // push tags onto the tag stack
        term : isize => 0isize, // name of the terminal
        termbidi | tbidi : isize => 0isize, // terminal takes care of bi-directionality
        terse : isize => 0isize, // shorten some messages
        textwidth | tw : isize => 0isize, // maximum width of text that is being inserted
        thesaurus | tsr : isize => 0isize, // list of thesaurus files for keyword completion
        thesaurusfunc | tsrfu : isize => 0isize, // function to be used for thesaurus completion
        tildeop | top : isize => 0isize, // tilde command "~" behaves like an operator
        timeout | to : isize => 0isize, // time out on mappings and key codes
        timeoutlen | tm : isize => 0isize, // time out time in milliseconds
        title : isize => 0isize, // let Vim set the title of the window
        titlelen : isize => 0isize, // percentage of 'columns' used for window title
        titleold : isize => 0isize, // old title, restored when exiting
        titlestring : isize => 0isize, // string to use for the Vim window title
        ttimeout : isize => 0isize, // time out on mappings
        ttimeoutlen | ttm : isize => 0isize, // time out time for key codes in milliseconds
        ttytype | tty : isize => 0isize, // alias for 'term'
        undodir | udir : isize => 0isize, // where to store undo files
        undofile | udf : isize => 0isize, // save undo information in a file
        undolevels | ul : isize => 0isize, // maximum number of changes that can be undone
        undoreload | ur : isize => 0isize, // max nr of lines to save for undo on a buffer reload
        updatecount | uc : isize => 0isize, // after this many characters flush swap file
        updatetime | ut : isize => 0isize, // after this many milliseconds flush swap file
        varsofttabstop | vsts : isize => 0isize, // a list of number of spaces when typing <Tab>
        vartabstop | vts : isize => 0isize, // a list of number of spaces for <Tab>s
        verbose | vbs : isize => 0isize, // give informative messages
        verbosefile | vfile : isize => 0isize, // file to write messages in
        viewdir | vdir : isize => 0isize, // directory where to store files with :mkview
        viewoptions | vop : isize => 0isize, // specifies what to save for :mkview
        virtualedit | ve : isize => 0isize, // when to use virtual editing
        visualbell | vb : isize => 0isize, // use visual bell instead of beeping
        warn : isize => 0isize, // warn for shell command when buffer was changed
        whichwrap | ww : isize => 0isize, // allow specified keys to cross line boundaries
        wildchar | wc : isize => 0isize, // command-line character for wildcard expansion
        wildcharm | wcm : isize => 0isize, // like 'wildchar' but also works when mapped
        wildignore | wig : isize => 0isize, // files matching these patterns are not completed
        wildignorecase | wic : isize => 0isize, // ignore case when completing file names
        wildmenu | wmnu : isize => 0isize, // use menu for command line completion
        wildmode | wim : isize => 0isize, // mode for 'wildchar' command-line expansion
        wildoptions | wop : isize => 0isize, // specifies how command line completion is done
        winaltkeys | wak : isize => 0isize, // when the windows system handles ALT keys
        window | wi : isize => 0isize, // nr of lines to scroll for CTRL-F and CTRL-B
        winheight | wh : isize => 0isize, // minimum number of lines for the current window
        winhighlight | winhl : isize => 0isize, // window-local highlighting
        winfixheight | wfh : isize => 0isize, // keep window height when opening/closing windows
        winfixwidth | wfw : isize => 0isize, // keep window width when opening/closing windows
        winminheight | wmh : isize => 0isize, // minimum number of lines for any window
        winminwidth | wmw : isize => 0isize, // minimal number of columns for any window
        winwidth | wiw : isize => 0isize, // minimal number of columns for current window
        wrap : isize => 0isize, // long lines wrap and continue on the next line
        wrapmargin | wm : isize => 0isize, // chars from the right where wrapping starts
        wrapscan | ws : isize => 0isize, // searches wrap around the end of the file
        write : bool => true, // writing to a file is allowed
        writeany | wa : bool => true, // write to file with no need for "!" override
        writebackup | wb : isize => 0isize, // make a backup before overwriting a file
        writedelay | wd : isize => 0isize, // delay this many msec for each char (for debug)
    }
}

options! {
    BufOptions {
        channel : isize => 0isize, // channel connected to buffer?

        autoindent | ai : bool => true, // take indent for new line from previous line
        autoread | ar : bool => true, // autom. read file when changed outside of Vim
        backupcopy | bkc : String => "auto", // make backup as a copy, don't rename the file
        binary | bin : bool => false, // read/write/edit file in binary mode
        belloff | bo : String => "all", // do not ring the bell for these reasons
        bufhidden | bh : String => "", // what to do when buffer is no longer in window
        buflisted | bl : bool => true, // whether the buffer shows up in the buffer list
        buftype | bt : String => "", // special type of buffer

        cindent | cin : bool => false, // do C program indenting
        cinkeys | cink : String => "0{,0},!^F,o,O,0[,0]", // keys that trigger indent when 'cindent' is set
        cinoptions | cino : String => "", // how to do indenting when 'cindent' is set
        cinwords | cinw : String => "for,if,else,while,loop,impl,mod,unsafe,trait,struct,enum,fn,extern", // words where 'si' and 'cin' add an indent
        cinscopedecls | cinsd : String => "public,protected,private", // words that are recognized by 'cino-g'

        comments | com : String => "s0:/*!,m: ,ex:*/,s1:/*,mb:*,ex:*/,:///,://!,://", // patterns that can start a comment line
        commentstring | cms : String => "//%s", // template for comments; used for fold marker
        complete | cpt : String => ".,w,b,u,t", // specify how Insert mode completion works
        completefunc | cfu : String => "", // function to be used for Insert mode completion
        completeslash | csl : String => "", // Overrules 'shellslash' for completion

        copyindent | ci : bool => false, // make 'autoindent' use existing indent structure
        dictionary | dict : String => "", // list of file names used for keyword completion

        endofline | eol : bool => true, // write <EOL> for last line in file
        equalprg | ep : String => "", // external program to use for "=" command
        errorformat | efm : String => "%*[^\"]\"%f\"%*\\D%l: %m,\"%f\"%*\\D%l: %m,%-G%f:%l: (Each undeclared identifier is reported only once,%-G%f:%l: for each function it appears in.),%-GIn file included from %f:%l:%c:,%-GIn file included from %f:%l:%c\\,,%-GIn file included from %f:%l:%c,%-GIn file included from %f:%l,%-G%*[ ]from %f:%l:%c,%-G%*[ ]from %f:%l:,%-G%*[ ]from %f:%l\\,,%-G%*[ ]from %f:%l,%f:%l:%c:%m,%f(%l):%m,%f:%l:%m,\"%f\"\\, line %l%*\\D%c%*[^ ] %m,%D%*\\a[%*\\d]: Entering directory %*[`']%f',%X%*\\a[%*\\d]: Leaving directory %*[`']%f',%D%*\\a: Entering directory %*[`']%f',%X%*\\a: Leaving directory %*[`']%f',%DMaking %*\\a in %f,%f|%l| %m", // description of the lines in the error file
        expandtab | et : bool => false, // use spaces when <Tab> is inserted
        // exrc | ex : isize => , // read .nvimrc and .exrc in the current directory
        fileencoding | fenc : String => "", // file encoding for multibyte text

        fileformat | ff : String => "unix", // file format used for file I/O
        fileformats | ffs : String => "unix", // automatically detected values for 'fileformat'

        filetype | ft : String => "", // type of file, used for autocommands
        fixendofline | fixeol : bool => true, // make sure last line in file has <EOL>
        foldtext | fdt : String => "foldtext()", // expression used to display for a closed fold
        formatlistpat | flp : String => "^\\s*\\d\\+[\\]:.)}\\t ]\\s*", // pattern used to recognize a list header
        formatoptions | fo : String => "tcqj", // how automatic formatting is to be done
        formatprg | fp : String => "", // name of external program used with "gq" command
        grepprg | gp : String => "grep -n ", // program to use for ":grep"
    }
}

options! {
    WinOptions {
        arabic | arab : bool => false, // for Arabic as a default second language
        breakindent | bri : bool => false, // wrapped line repeats indent
        breakindentopt | briopt : bool => false, // settings for 'breakindent'
        colorcolumn | cc : String => "", // columns to highlight

        concealcursor | cocu : String => "", // whether concealable text is hidden in cursor line
        conceallevel | cole : isize => 0isize, // whether concealable text is shown or hidden

        cursorbind | crb : bool => false, // move cursor in window as it moves in other windows
        cursorcolumn | cuc : bool => false, // highlight the screen column of the cursor
        cursorline | cul : bool => false, // highlight the screen line of the cursor
        cursorlineopt | culopt : String => "both", // settings for 'cursorline'
        diff : bool => false, // use diff mode for the current window
        fillchars | fcs : String => "", // characters to use for displaying special items

        foldcolumn | fdc : isize => 0isize, // width of the column used to indicate folds
        foldenable | fen : bool => true, // set to display all folds open
        foldexpr | fde : String => "0", // expression used when 'foldmethod' is "expr"
        foldignore | fdi : String => "#", // ignore lines when 'foldmethod' is "indent"
        foldlevel | fdl : isize => 0isize, // close folds with a level higher than this

        foldmarker | fmr : String => "{{{,}}}", // markers used when 'foldmethod' is "marker"
        foldmethod | fdm : String => "manual", // folding type
        foldminlines | fml : isize => 1isize, // minimum number of lines for a fold to be closed
        foldnestmax | fdn : isize => 20isize, // maximum fold depth
        foldopen | fdo : String => "block,hor,mark,percent,quickfix,search,tag,undo", // for which commands a fold will be opened
    }
}
