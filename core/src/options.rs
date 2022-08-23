use bitfield::bitfield;
use std::borrow::Cow;
use std::str::FromStr;
use vimscript::{CmdRange, State, Value, ValueRef, VimError, VimScriptCtx};

use crate::VimInner;

macro_rules! str_enum {
    (enum $name:ident { $($var:ident $( = $alt:literal)?),* $(,)?}) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
        pub enum $name {
            $($var,)*
        }

        impl $name {
            const VALUES: &'static [&'static str] = &[$(stringify!($var),)*];
        }

        impl FromStr for $name {
            type Err = VimError;
            fn from_str(v: &str) -> Result<Self, Self::Err> {
                match v {
                    $(stringify!($var) $(| $alt)? => Ok(Self::$var),)*
                    _ => Err(VimError::ExpectedOne(Self::VALUES)),
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

        impl Into<ValueRef<'static>> for $name {
            fn into(self) -> ValueRef<'static> {
                <Self as Into<&'static str>>::into(self).into()
            }
        }

        impl FromBool for $name {
            fn from_bool(_b: bool) -> Result<Self, VimError> {
                Err(VimError::NotABool)
            }
        }
    };
    (struct $name:ident { $($var:ident; $set:ident: $num:literal $( = $alt:literal)?),* $(,)?}) => {
        bitfield! {
            #[derive(Clone, Copy, Hash, PartialEq, Eq)]
            pub struct $name(u64);
            impl Debug;
            $(pub $var, $set: $num;)*
        }

        impl $name {
            const VALUES: &'static [&'static str] = &[$(stringify!($var),)*];
        }

        impl FromStr for $name {
            type Err = VimError;
            fn from_str(v: &str) -> Result<Self, Self::Err> {
                let mut ret = Self(0);
                for name in v.split(',') {
                    match name {
                        $(stringify!($var) => ret.$set(true),)*
                        _ => return Err(VimError::ExpectedMany(Self::VALUES)),
                    }
                }
                Ok(ret)
            }
        }

        impl Into<ValueRef<'static>> for $name {
            fn into(self) -> ValueRef<'static> {
                let mut ret = String::new();
                $(if self.$var() { ret += concat!(stringify!($var), ",") })*
                ValueRef::Str(Cow::Owned(ret))
            }
        }

        impl FromBool for $name {
            fn from_bool(_b: bool) -> Result<Self, VimError> {
                Err(VimError::NotABool)
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

str_enum!(struct BellOff {
    all; set_all: 0,
    backspace; set_backspace: 1,
    cursor; set_cursor: 2,
    complete; set_complete: 3,
    copy; set_copy: 4,
    ctrlg; set_ctrlg: 5,
    error; set_error: 6,
    esc; set_esc: 7,
    hangul; set_hangul: 8,
    insertmode; set_insertmode: 9,
    lang; set_lang: 10,
    mess; set_mess: 11,
    showmatch; set_showmatch: 12,
    operator; set_operator: 13,
    register; set_register: 14,
    shell; set_shell: 15,
    spell; set_spell: 16,
    wildmode; set_wildmode: 17,
});

impl Default for BellOff {
    fn default() -> Self {
        let mut ret = Self(0);
        ret.set_all(true);
        ret
    }
}

fn list_options_non_default<O: Opts>(opts: &O) -> String {
    let mut ret = String::new();
    for name in O::list() {
        if !opts.is_default(name).unwrap() {
            use std::fmt::Write;
            ret.write_fmt(format_args!("{} = {}", name, opts.get(name).unwrap()))
                .unwrap();
        }
    }
    ret
}

fn list_options<O: Opts>(opts: &O) -> String {
    let mut ret = String::new();
    for name in O::list() {
        use std::fmt::Write;
        ret.write_fmt(format_args!("{} = {}", name, opts.get(name).unwrap()))
            .unwrap();
    }
    ret
}

fn set_option_part(args: &str, opts: &mut impl Opts) -> Result<Option<String>, String> {
    if args.trim() == "all" {
        Ok(Some(list_options(opts)))
    } else if let Some(name) = args.trim().strip_suffix('?') {
        if let Ok(v) = opts.get(name) {
            Ok(Some(format!("{}", v)))
        } else {
            Err(format!("{name} is not a valid option"))
        }
    } else if let Some(name) = args
        .trim()
        .strip_suffix('!')
        .or_else(|| args.trim().strip_prefix("inv"))
    {
        if let Ok(ValueRef::Bool(b)) = opts.get(name) {
            // Since get worked & retuned a bool, this is fine
            opts.set_bool(name, !b).unwrap();
            Ok(None)
        } else {
            Err(format!("{name} is not a boolean"))
        }
    } else if let Some(name) = args.trim().strip_prefix("no") {
        match opts.set_bool(name, false) {
            Ok(()) => Ok(None),
            Err(VimError::NotABool) => Ok(Some(format!("{name} is not a boolean"))),
            Err(e) => Err(format!("{name} is not defined")),
        }
    } else if let Some((name, value)) = args.split_once('=') {
        match opts.set(name, value) {
            Ok(()) => Ok(None),
            Err(e) => Err(format!("{name} is not defined")),
        }
    } else {
        match opts.set_bool(args.trim(), true) {
            Ok(()) => Ok(None),
            Err(VimError::NotABool) => {
                if let Ok(v) = opts.get(args.trim()) {
                    Ok(Some(format!("{}", v)))
                } else {
                    Err(format!("{args} is not a valid option"))
                }
            }
            Err(e) => Err(format!("{args} is not defined")),
        }
    }
}

pub(crate) fn set_option(
    _range: CmdRange<'_>,
    _bang: bool,
    args: &str,
    _ctx: &mut VimScriptCtx<VimInner>,
    state: &mut VimInner,
) {
    if args.trim() == "" {
        state.message(list_options_non_default(state.options()));
        state.message(list_options_non_default(state.get_focus().options()));
        state.message(
            state
                .get_focus()
                .buffer()
                .with_read(|b| list_options_non_default(b.options())),
        );
    } else {
        let mut last = ' ';
        for args in args.split(|c: char| {
            let ret = c.is_whitespace() && last != '\\';
            last = c;
            ret
        }) {
            match set_option_part(args, state.options_mut()) {
                Ok(Some(s)) => state.message(s),
                Ok(None) => (),
                Err(_) => match set_option_part(args, state.get_focus_mut().options_mut()) {
                    Ok(Some(s)) => state.message(s),
                    Ok(None) => (),
                    Err(_) => {
                        match state
                            .get_focus()
                            .buffer()
                            .with_write(|b| set_option_part(args, b.options_mut()))
                        {
                            Ok(Some(s)) | Err(s) => state.message(s),
                            Ok(None) => (),
                        }
                    }
                },
            }
        }
    }
}

pub(crate) fn set_local(
    _range: CmdRange<'_>,
    _bang: bool,
    args: &str,
    _ctx: &mut VimScriptCtx<VimInner>,
    state: &mut VimInner,
) {
    if args.trim() == "" {
        state.message(list_options_non_default(state.get_focus().options()));
        state.message(
            state
                .get_focus()
                .buffer()
                .with_read(|b| list_options_non_default(b.options())),
        );
    } else {
        let mut last = ' ';
        for args in args.split(|c: char| {
            let ret = c.is_whitespace() && last != '\\';
            last = c;
            ret
        }) {
            match set_option_part(args, state.get_focus_mut().options_mut()) {
                Ok(Some(s)) => state.message(s),
                Ok(None) => (),
                Err(_) => {
                    match state
                        .get_focus()
                        .buffer()
                        .with_write(|b| set_option_part(args, b.options_mut()))
                    {
                        Ok(Some(s)) | Err(s) => state.message(s),
                        Ok(None) => (),
                    }
                }
            }
        }
    }
}

pub(crate) fn set_global(
    _range: CmdRange<'_>,
    _bang: bool,
    args: &str,
    _ctx: &mut VimScriptCtx<VimInner>,
    state: &mut VimInner,
) {
    if args.trim() == "" {
        state.message(list_options_non_default(state.options()));
    } else {
        let mut last = ' ';
        for args in args.split(|c: char| {
            let ret = c.is_whitespace() && last != '\\';
            last = c;
            ret
        }) {
            match set_option_part(args, state.options_mut()) {
                Ok(Some(s)) | Err(s) => state.message(s),
                Ok(None) => (),
            }
        }
    }
}

pub trait Opts {
    fn new() -> Self;
    fn get<'s>(&'s self, name: &str) -> Result<ValueRef<'s>, VimError>;
    fn set(&mut self, name: &str, val: &str) -> Result<(), VimError>;
    fn set_bool(&mut self, name: &str, val: bool) -> Result<(), VimError>;
    fn list() -> &'static [&'static str];
    fn default_value(name: &str) -> Option<&'static str>;
    fn is_default(&self, name: &str) -> Option<bool>;
}

trait FromBool: Sized {
    fn from_bool(b: bool) -> Result<Self, VimError>;
}

impl FromBool for bool {
    fn from_bool(b: bool) -> Result<Self, VimError> {
        Ok(b)
    }
}

impl FromBool for String {
    fn from_bool(_b: bool) -> Result<Self, VimError> {
        Err(VimError::NotABool)
    }
}

impl FromBool for isize {
    fn from_bool(_b: bool) -> Result<Self, VimError> {
        Err(VimError::NotABool)
    }
}

macro_rules! options {
    ($opts:ident {$($name1:ident $(| $name2:ident $(| $name3:ident)?)? : $ty:ty => $val:literal),* $(,)?}) => {
        #[derive(Debug, Clone)]
        pub struct $opts {
            $(pub $name1: $ty,)*
        }

        impl Opts for $opts {
            fn new() -> Self {
                Self {
                    $($name1: $val.parse::<$ty>().unwrap(),)*
                }
            }

            fn get<'s>(&'s self, name: &str) -> Result<ValueRef<'s>, VimError> {
                match name {
                    $(stringify!($name1) $(| stringify!($name2) $(| stringify!($name3))?)?  => Ok((&self.$name1).into()),)*
                    _ => Err(VimError::VariableUndefined(name.to_string()))
                }
            }

            fn set(&mut self, name: &str, val: &str) -> Result<(), VimError> {
                match name {
                    $(stringify!($name1) $(| stringify!($name2) $(| stringify!($name3))?)?  => self.$name1 = val.parse::<$ty>()?,)*
                    _ => return Err(VimError::VariableUndefined(name.to_string())),
                }
                Ok(())
            }

            fn set_bool(&mut self, name: &str, val: bool) -> Result<(), VimError> {
                match name {
                    $(stringify!($name1) $(| stringify!($name2) $(| stringify!($name3))?)?  => self.$name1 = <$ty as FromBool>::from_bool(val)?,)*
                    _ => return Err(VimError::VariableUndefined(name.to_string())),
                }
                Ok(())
            }

            fn list() -> &'static [&'static str] {
                &[$(stringify!($name1),)*]
            }

            fn default_value(name: &str) -> Option<&'static str> {
                match name {
                    $(stringify!($name1) => Some($val),)*
                    _ => None,
                }
            }

            fn is_default(&self, name: &str) -> Option<bool> {
                match name {
                    $(stringify!($name1) => Some($val.parse::<$ty>().unwrap() == self.$name1),)*
                    _ => None,
                }
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
        aleph | al : isize => "224", // ASCII code of the letter Aleph (Hebrew)
        allowrevins | ari : bool => "false", // allow CTRL-_ in Insert and Command-line mode
        ambiwidth | ambw : String => "single", // what to do with Unicode chars of ambiguous width
        autochdir | acd : bool => "false", // change directory to the file in the current window
        arabicshape | arshape : bool => "true", // do shaping for Arabic characters
        autoread | ar : bool => "true", // autom. read file when changed outside of Vim
        autowrite | aw : bool => "false", // automatically write file if changed
        autowriteall | awa : bool => "false", // as 'autowrite', but works with more commands
        background | bg : String => "dark", // "dark" or "light", used for highlight colors
        backspace | bs : String => "indent,eol,start,nostop", // how backspace works at start of line
        backup | bk : bool => "false", // keep backup file after overwriting a file
        backupcopy | bkc : String => "auto", // make backup as a copy, don't rename the file
        backupdir | bdir : String => ".,/home/matthew/.local/share/nvim/backup//", // list of directories for the backup file
        backupext | bex : String => "~", // extension used for the backup file
        backupskip | bsk : String => "/tmp/*", // no backup for files that match these patterns
        bomb : bool => "false", // prepend a Byte Order Mark to the file
        breakat | brk : String => "     !@*-+;:,./?", // characters that may cause a line break
        browsedir | bsdir : String => "last", // which directory to start browsing in
        casemap | cmp : String => "internal,keepascii", // specifies how case of letters is changed
        cdhome | cdh : bool => "false", // change directory to the home directory by ":cd"
        cdpath | cd : String => ",,", // list of directories searched with ":cd"
        cedit : String => "", // key used to open the command-line window
        charconvert | ccv : String => "", // expression for character encoding conversion
        clipboard | cb : String => "unnamedplus", // use the clipboard as the unnamed register
        cmdheight | ch : isize => "1", // number of lines to use for the command-line
        cmdwinheight | cwh : isize => "7", // height of the command-line window
        columns | co : isize => "80", // number of columns in the display
        completeopt | cot : String => "menuone,noselect", // options for Insert mode completion
        confirm | cf : bool => "false", // ask what to do about unsaved/read-only files
        cpoptions | cpo : String => "aABceFs_", // flags for Vi-compatible behavior
        cscopepathcomp | cspc : isize => "0", // how many components of the path to show
        cscopeprg | csprg : String => "cscope", // command to execute cscope
        cscopequickfix | csqf : String => "", // use quickfix window for cscope results
        cscoperelative | csre : bool => "false", // Use cscope.out path basename as prefix
        cscopetag | cst : bool => "true", // use cscope for tag commands
        cscopetagorder | csto : isize => "0", // determines ":cstag" search order
        debug : String => "", // set to "msg" to see all error messages
        define | def : String => "^\\s*#\\s*define", // pattern to be used to find a macro definition
        delcombine | deco : bool => "false", // delete combining characters on their own
        dictionary | dict : String => "", // list of file names used for keyword completion
        diffexpr | dex : String => "", // expression used to obtain a diff file
        diffopt | dip : String => "internal,filler,closeoff", // options for using diff mode
        digraph | dg : bool => "false", // enable the entering of digraphs in Insert mode
        directory | dir : String => "/home/matthew/.local/share/nvim/swap//", // list of directory names for the swap file
        display | dy : String => "lastline,msgsep", // list of flags for how to display text
        eadirection | ead : String => "both", // in which direction 'equalalways' works
        emoji | emo : bool => "true",
        encoding | enc : String => "UTF-8", // encoding used internally
        equalalways | ea : bool => "true", // windows are automatically made the same size
        equalprg | ep : String => "", // external program to use for "=" command
        errorbells | eb : bool => "false", // ring the bell for error messages
        errorfile | ef : String => "errors.err", // name of the errorfile for the QuickFix mode
        errorformat | efm : String => "%*[^\"]\"%f\"%*\\D%l: %m,\"%f\"%*\\D%l: %m,%-G%f:%l: (Each undeclared identifier is reported only once,%-G%f:%l: for each function it appears in.),%-GIn file included from %f:%l:%c:,%-GIn file included from %f:%l:%c\\,,%-GIn file included from %f:%l:%c,%-GIn file included from %f:%l,%-G%*[ ]from %f:%l:%c,%-G%*[ ]from %f:%l:,%-G%*[ ]from %f:%l\\,,%-G%*[ ]from %f:%l,%f:%l:%c:%m,%f(%l):%m,%f:%l:%m,\"%f\"\\, line %l%*\\D%c%*[^ ] %m,%D%*\\a[%*\\d]: Entering directory %*[`']%f',%X%*\\a[%*\\d]: Leaving directory %*[`']%f',%D%*\\a: Entering directory %*[`']%f',%X%*\\a: Leaving directory %*[`']%f',%DMaking %*\\a in %f,%f|%l| %m", // description of the lines in the error file
        eventignore | ei : String => "", // autocommand events that are ignored
        fileencodings | fencs : String => "ucs-bom,utf-8,default,latin1", // automatically detected character encodings
        fileignorecase | fic : bool => "false", // ignore case when using file names
        fillchars | fcs : String => "", // characters to use for displaying special items
        foldclose | fcl : String => "", // close a fold when the cursor leaves it
        foldlevelstart | fdls : isize => "-1", // when starting to edit a file
        formatexpr | fex : String => "", // expression used with "gq" command
        formatprg | fp : String => "", // name of external program used with "gq" command
        fsync | fs : bool => "false", // whether to invoke fsync() after file write
        gdefault | gd : bool => "false", // the ":substitute" flag 'g' is default on
        grepformat | gfm : String => "%f:%l:%m,%f:%l%m,%f  %l%m", // format of 'grepprg' output
        grepprg | gp : String => "grep -n ", // program to use for ":grep"
        guicursor | gcr : String => "n-v-c-sm:block,i-ci-ve:ver25,r-cr-o:hor20", // GUI: settings for cursor shape and blinking
        guifont | gfn : String => "", // GUI: Name(s) of font(s) to be used
        guifontwide | gfw : String => "", // list of font names for double-wide characters
        // guioptions | go : String => "", // GUI: Which components and options are used
        guitablabel | gtl : String => "", // GUI: custom label for a tab page
        guitabtooltip | gtt : isize => "0", // GUI: custom tooltip for a tab page
        helpfile | hf : isize => "0", // full path name of the main help file
        helpheight | hh : isize => "0", // minimum height of a new help window
        helplang | hlg : isize => "0", // preferred help languages
        hidden | hid : isize => "0", // don't unload buffer when it is |abandon|ed
        hlsearch | hls : isize => "0", // highlight matches with last search pattern
        history | hi : isize => "0", // number of command-lines that are remembered
        hkmap | hk : isize => "0", // Hebrew keyboard mapping
        hkmapp | hkp : isize => "0", // phonetic Hebrew keyboard mapping
        icon : isize => "0", // let Vim set the text of the window icon
        iconstring : isize => "0", // string to use for the Vim icon text
        ignorecase | ic : isize => "0", // ignore case in search patterns
        imcmdline | imc : isize => "0", // use IM when starting to edit a command line
        imdisable | imd : isize => "0", // do not use the IM in any mode
        iminsert | imi : isize => "0", // use :lmap or IM in Insert mode
        imsearch | ims : isize => "0", // use :lmap or IM when typing a search pattern
        include | inc : isize => "0", // pattern to be used to find an include file
        includeexpr | inex : isize => "0", // expression used to process an include line
        incsearch | is : isize => "0", // highlight match while typing search pattern
        indentexpr | inde : isize => "0", // expression used to obtain the indent of a line
        indentkeys | indk : isize => "0", // keys that trigger indenting with 'indentexpr'
        infercase | inf : isize => "0", // adjust case of match for keyword completion
        insertmode | im : isize => "0", // start the edit of a file in Insert mode
        isfname | isf : isize => "0", // characters included in file names and pathnames
        isident | isi : isize => "0", // characters included in identifiers
        iskeyword | isk : isize => "0", // characters included in keywords
        isprint | isp : isize => "0", // printable characters
        joinspaces | js : isize => "0", // two spaces after a period with a join command
        jumpoptions | jop : isize => "0", // specifies how jumping is done
        keymap | kmp : isize => "0", // name of a keyboard mapping
        keymodel | km : isize => "0", // enable starting/stopping selection with keys
        keywordprg | kp : isize => "0", // program to use for the "K" command
        langmap | lmap : isize => "0", // alphabetic characters for other language mode
        langmenu | lm : isize => "0", // language to be used for the menus
        langremap | lrm : isize => "0", // do apply 'langmap' to mapped characters
        laststatus | ls : isize => "0", // tells when last window has status lines
        lazyredraw | lz : isize => "0", // don't redraw while executing macros
        linebreak | lbr : isize => "0", // wrap long lines at a blank
        lines : isize => "0", // number of lines in the display
        linespace | lsp : isize => "0", // number of pixel lines to use between characters
        lisp : isize => "0", // automatic indenting for Lisp
        lispwords | lw : isize => "0", // words that change how lisp indenting works
        list : isize => "0", // show <Tab> and <EOL>
        listchars | lcs : isize => "0", // characters for displaying in list mode
        loadplugins | lpl : isize => "0", // load plugin scripts when starting up
        magic : isize => "0", // changes special characters in search patterns
        makeef | mef : isize => "0", // name of the errorfile for ":make"
        makeencoding | menc : isize => "0", // encoding of external make/grep commands
        makeprg | mp : isize => "0", // program to use for the ":make" command
        matchpairs | mps : isize => "0", // pairs of characters that "%" can match
        matchtime | mat : isize => "0", // tenths of a second to show matching paren
        maxcombine | mco : isize => "0", // maximum nr of combining characters displayed
        maxfuncdepth | mfd : isize => "0", // maximum recursive depth for user functions
        maxmapdepth | mmd : isize => "0", // maximum recursive depth for mapping
        maxmempattern | mmp : isize => "0", // maximum memory (in Kbyte) used for pattern search
        menuitems | mis : isize => "0", // maximum number of items in a menu
        mkspellmem | msm : isize => "0", // memory used before |:mkspell| compresses the tree
        modeline | ml : isize => "0", // recognize modelines at start or end of file
        modelineexpr | mle : isize => "0", // allow setting expression options from a modeline
        modelines | mls : isize => "0", // number of lines checked for modelines
        modifiable | ma : isize => "0", // changes to the text are not possible
        modified | mod : isize => "0", // buffer has been modified
        more : isize => "0", // pause listings when the whole screen is filled
        mouse : isize => "0", // enable the use of mouse clicks
        mousefocus | mousef : isize => "0", // keyboard focus follows the mouse
        mousehide | mh : isize => "0", // hide mouse pointer while typing
        mousemodel | mousem : isize => "0", // changes meaning of mouse buttons
        mouseshape | mouses : isize => "0", // shape of the mouse pointer in different modes
        mousetime | mouset : isize => "0", // max time between mouse double-click
        nrformats | nf : isize => "0", // number formats recognized for CTRL-A command
        number | nu : isize => "0", // print the line number in front of each line
        numberwidth | nuw : isize => "0", // number of columns used for the line number
        omnifunc | ofu : isize => "0", // function for filetype-specific completion
        opendevice | odev : isize => "0", // allow reading/writing devices on MS-Windows
        operatorfunc | opfunc : isize => "0", // function to be called for |g@| operator
        packpath | pp : isize => "0", // list of directories used for packages
        paragraphs | para : isize => "0", // nroff macros that separate paragraphs
        paste : isize => "0", // allow pasting text
        pastetoggle | pt : isize => "0", // key code that causes 'paste' to toggle
        patchexpr | pex : isize => "0", // expression used to patch a file
        patchmode | pm : isize => "0", // keep the oldest version of a file
        path | pa : isize => "0", // list of directories searched with "gf" et.al.
        perldll : isize => "0", // name of the Perl dynamic library
        preserveindent | pi : isize => "0", // preserve the indent structure when reindenting
        previewheight | pvh : isize => "0", // height of the preview window
        previewpopup | pvp : isize => "0", // use popup window for preview
        previewwindow | pvw : isize => "0", // identifies the preview window
        printdevice | pdev : isize => "0", // name of the printer to be used for :hardcopy
        printencoding | penc : isize => "0", // encoding to be used for printing
        printexpr | pexpr : isize => "0", // expression used to print PostScript for :hardcopy
        printfont | pfn : isize => "0", // name of the font to be used for :hardcopy
        printheader | pheader : isize => "0", // format of the header used for :hardcopy
        printmbcharset | pmbcs : isize => "0", // CJK character set to be used for :hardcopy
        printmbfont | pmbfn : isize => "0", // font names to be used for CJK output of :hardcopy
        printoptions | popt : isize => "0", // controls the format of :hardcopy output
        pumheight | ph : isize => "0", // maximum height of the popup menu
        pumwidth | pw : isize => "0", // minimum width of the popup menu
        pythondll : isize => "0", // name of the Python 2 dynamic library
        pythonthreedll : isize => "0", // name of the Python 3 dynamic library
        pyxversion | pyx : isize => "0", // Python version used for pyx* commands
        quoteescape | qe : isize => "0", // escape characters used in a string
        readonly | ro : isize => "0", // disallow writing the buffer
        redrawtime | rdt : isize => "0", // timeout for 'hlsearch' and |:match| highlighting
        regexpengine | re : isize => "0", // default regexp engine to use
        relativenumber | rnu : isize => "0", // show relative line number in front of each line
        remap : isize => "0", // allow mappings to work recursively
        report : isize => "0", // threshold for reporting nr. of lines changed
        revins | ri : isize => "0", // inserting characters will work backwards
        rightleft | rl : isize => "0", // window is right-to-left oriented
        rightleftcmd | rlc : isize => "0", // commands for which editing works right-to-left
        rubydll : isize => "0", // name of the Ruby dynamic library
        ruler | ru : bool => "false", // show cursor line and column in the status line
        rulerformat | ruf : isize => "0", // custom format for the ruler
        runtimepath | rtp : String => "$XDG_CONFIG_HOME/rvim/", // list of directories used for runtime files
        scroll | scr : isize => "1", // lines to scroll with CTRL-U and CTRL-D
        scrollbind | scb : isize => "0", // scroll in window as other windows scroll
        scrolljump | sj : isize => "0", // minimum number of lines to scroll
        scrolloff | so : isize => "0", // minimum nr. of lines above and below cursor
        scrollopt | sbo : isize => "0", // how 'scrollbind' should behave
        sections | sect : isize => "0", // nroff macros that separate sections
        secure : isize => "0", // secure mode for reading .vimrc in current dir
        selection | sel : isize => "0", // what type of selection to use
        selectmode | slm : isize => "0", // when to use Select mode instead of Visual mode
        sessionoptions | ssop : isize => "0", // options for |:mksession|
        shada | sd : isize => "0", // use .shada file upon startup and exiting
        shell | sh : isize => "0", // name of shell to use for external commands
        shellcmdflag | shcf : isize => "0", // flag to shell to execute one command
        shellpipe | sp : isize => "0", // string to put output of ":make" in error file
        shellquote | shq : isize => "0", // quote character(s) for around shell command
        shellredir | srr : isize => "0", // string to put output of filter in a temp file
        shellslash | ssl : isize => "0", // use forward slash for shell file names
        shelltemp | stmp : isize => "0", // whether to use a temp file for shell commands
        shellxescape | sxe : isize => "0", // characters to escape when 'shellxquote' is (
        shellxquote | sxq : isize => "0", // like 'shellquote', but include redirection
        shiftround | sr : isize => "0", // round indent to multiple of shiftwidth
        shiftwidth | sw : isize => "0", // number of spaces to use for (auto)indent step
        shortmess | shm : isize => "0", // list of flags, reduce length of messages
        showbreak | sbr : isize => "0", // string to use at the start of wrapped lines
        showcmd | sc : isize => "0", // show (partial) command in status line
        showfulltag | sft : isize => "0", // show full tag pattern when completing tag
        showmatch | sm : isize => "0", // briefly jump to matching bracket if insert one
        showmode | smd : isize => "0", // message on status line to show current mode
        showtabline | stal : isize => "0", // tells when the tab pages line is displayed
        sidescroll | ss : isize => "0", // minimum number of columns to scroll horizontal
        sidescrolloff | siso : isize => "0", // min. nr. of columns to left and right of cursor
        signcolumn | scl : isize => "0", // when and how to display the sign column
        smartcase | scs : isize => "0", // no ignore case when pattern has uppercase
        smartindent | si : isize => "0", // smart autoindenting for C programs
        smarttab | sta : isize => "0", // use 'shiftwidth' when inserting <Tab>
        softtabstop | sts : isize => "0", // number of spaces that <Tab> uses while editing
        spell : isize => "0", // enable spell checking
        spellcapcheck | spc : isize => "0", // pattern to locate end of a sentence
        spellfile | spf : isize => "0", // files where |zg| and |zw| store words
        spelllang | spl : isize => "0", // language(s) to do spell checking for
        spelloptions | spo : isize => "0", // options for spell checking
        spellsuggest | sps : isize => "0", // method(s) used to suggest spelling corrections
        splitbelow | sb : isize => "0", // new window from split is below the current one
        splitright | spr : isize => "0", // new window is put right of the current one
        startofline | sol : isize => "0", // commands move cursor to first non-blank in line
        statusline | stl : isize => "0", // custom format for the status line
        suffixes | su : isize => "0", // suffixes that are ignored with multiple match
        suffixesadd | sua : isize => "0", // suffixes added when searching for a file
        swapfile | swf : isize => "0", // whether to use a swapfile for a buffer
        switchbuf | swb : isize => "0", // sets behavior when switching to another buffer
        synmaxcol | smc : isize => "0", // maximum column to find syntax items
        syntax | syn : isize => "0", // syntax to be loaded for current buffer
        tabline | tal : isize => "0", // custom format for the console tab pages line
        tabpagemax | tpm : isize => "0", // maximum number of tab pages for |-p| and "tab all"
        tabstop | ts : isize => "0", // number of spaces that <Tab> in file uses
        tagbsearch | tbs : isize => "0", // use binary searching in tags files
        tagcase | tc : isize => "0", // how to handle case when searching in tags files
        taglength | tl : isize => "0", // number of significant characters for a tag
        tagrelative | tr : isize => "0", // file names in tag file are relative
        tags | tag : isize => "0", // list of file names used by the tag command
        tagstack | tgst : isize => "0", // push tags onto the tag stack
        term : isize => "0", // name of the terminal
        termbidi | tbidi : isize => "0", // terminal takes care of bi-directionality
        terse : isize => "0", // shorten some messages
        textwidth | tw : isize => "0", // maximum width of text that is being inserted
        thesaurus | tsr : isize => "0", // list of thesaurus files for keyword completion
        thesaurusfunc | tsrfu : isize => "0", // function to be used for thesaurus completion
        tildeop | top : isize => "0", // tilde command "~" behaves like an operator
        timeout | to : isize => "0", // time out on mappings and key codes
        timeoutlen | tm : isize => "0", // time out time in milliseconds
        title : isize => "0", // let Vim set the title of the window
        titlelen : isize => "0", // percentage of 'columns' used for window title
        titleold : isize => "0", // old title, restored when exiting
        titlestring : isize => "0", // string to use for the Vim window title
        ttimeout : isize => "0", // time out on mappings
        ttimeoutlen | ttm : isize => "0", // time out time for key codes in milliseconds
        ttytype | tty : isize => "0", // alias for 'term'
        undodir | udir : isize => "0", // where to store undo files
        undofile | udf : isize => "0", // save undo information in a file
        undolevels | ul : isize => "0", // maximum number of changes that can be undone
        undoreload | ur : isize => "0", // max nr of lines to save for undo on a buffer reload
        updatecount | uc : isize => "0", // after this many characters flush swap file
        updatetime | ut : isize => "0", // after this many milliseconds flush swap file
        varsofttabstop | vsts : isize => "0", // a list of number of spaces when typing <Tab>
        vartabstop | vts : isize => "0", // a list of number of spaces for <Tab>s
        verbose | vbs : isize => "0", // give informative messages
        verbosefile | vfile : isize => "0", // file to write messages in
        viewdir | vdir : isize => "0", // directory where to store files with :mkview
        viewoptions | vop : isize => "0", // specifies what to save for :mkview
        virtualedit | ve : isize => "0", // when to use virtual editing
        visualbell | vb : isize => "0", // use visual bell instead of beeping
        warn : isize => "0", // warn for shell command when buffer was changed
        whichwrap | ww : isize => "0", // allow specified keys to cross line boundaries
        wildchar | wc : isize => "0", // command-line character for wildcard expansion
        wildcharm | wcm : isize => "0", // like 'wildchar' but also works when mapped
        wildignore | wig : isize => "0", // files matching these patterns are not completed
        wildignorecase | wic : isize => "0", // ignore case when completing file names
        wildmenu | wmnu : isize => "0", // use menu for command line completion
        wildmode | wim : isize => "0", // mode for 'wildchar' command-line expansion
        wildoptions | wop : isize => "0", // specifies how command line completion is done
        winaltkeys | wak : isize => "0", // when the windows system handles ALT keys
        window | wi : isize => "0", // nr of lines to scroll for CTRL-F and CTRL-B
        winheight | wh : isize => "0", // minimum number of lines for the current window
        winhighlight | winhl : isize => "0", // window-local highlighting
        winfixheight | wfh : isize => "0", // keep window height when opening/closing windows
        winfixwidth | wfw : isize => "0", // keep window width when opening/closing windows
        winminheight | wmh : isize => "0", // minimum number of lines for any window
        winminwidth | wmw : isize => "0", // minimal number of columns for any window
        winwidth | wiw : isize => "0", // minimal number of columns for current window
        wrap : isize => "0", // long lines wrap and continue on the next line
        wrapmargin | wm : isize => "0", // chars from the right where wrapping starts
        wrapscan | ws : isize => "0", // searches wrap around the end of the file
        write : bool => "true", // writing to a file is allowed
        writeany | wa : bool => "true", // write to file with no need for "!" override
        writebackup | wb : isize => "0", // make a backup before overwriting a file
        writedelay | wd : isize => "0", // delay this many msec for each char (for debug)
    }
}

options! {
    BufOptions {
        channel : isize => "0", // channel connected to buffer?

        autoindent | ai : bool => "true", // take indent for new line from previous line
        autoread | ar : bool => "true", // autom. read file when changed outside of Vim
        backupcopy | bkc : String => "auto", // make backup as a copy, don't rename the file
        binary | bin : bool => "false", // read/write/edit file in binary mode
        belloff | bo : BellOff => "all", // do not ring the bell for these reasons
        bufhidden | bh : BufHidden => "", // what to do when buffer is no longer in window
        buflisted | bl : bool => "true", // whether the buffer shows up in the buffer list
        buftype | bt : String => "", // special type of buffer

        cindent | cin : bool => "false", // do C program indenting
        cinkeys | cink : String => "0{,0},!^F,o,O,0[,0]", // keys that trigger indent when 'cindent' is set
        cinoptions | cino : String => "", // how to do indenting when 'cindent' is set
        cinwords | cinw : String => "for,if,else,while,loop,impl,mod,unsafe,trait,struct,enum,fn,extern", // words where 'si' and 'cin' add an indent
        cinscopedecls | cinsd : String => "public,protected,private", // words that are recognized by 'cino-g'

        comments | com : String => "s0:/*!,m: ,ex:*/,s1:/*,mb:*,ex:*/,:///,://!,://", // patterns that can start a comment line
        commentstring | cms : String => "//%s", // template for comments; used for fold marker
        complete | cpt : String => ".,w,b,u,t", // specify how Insert mode completion works
        completefunc | cfu : String => "", // function to be used for Insert mode completion
        completeslash | csl : String => "", // Overrules 'shellslash' for completion

        copyindent | ci : bool => "false", // make 'autoindent' use existing indent structure
        dictionary | dict : String => "", // list of file names used for keyword completion

        endofline | eol : bool => "true", // write <EOL> for last line in file
        equalprg | ep : String => "", // external program to use for "=" command
        errorformat | efm : String => "%*[^\"]\"%f\"%*\\D%l: %m,\"%f\"%*\\D%l: %m,%-G%f:%l: (Each undeclared identifier is reported only once,%-G%f:%l: for each function it appears in.),%-GIn file included from %f:%l:%c:,%-GIn file included from %f:%l:%c\\,,%-GIn file included from %f:%l:%c,%-GIn file included from %f:%l,%-G%*[ ]from %f:%l:%c,%-G%*[ ]from %f:%l:,%-G%*[ ]from %f:%l\\,,%-G%*[ ]from %f:%l,%f:%l:%c:%m,%f(%l):%m,%f:%l:%m,\"%f\"\\, line %l%*\\D%c%*[^ ] %m,%D%*\\a[%*\\d]: Entering directory %*[`']%f',%X%*\\a[%*\\d]: Leaving directory %*[`']%f',%D%*\\a: Entering directory %*[`']%f',%X%*\\a: Leaving directory %*[`']%f',%DMaking %*\\a in %f,%f|%l| %m", // description of the lines in the error file
        expandtab | et : bool => "false", // use spaces when <Tab> is inserted
        // exrc | ex : isize => , // read .nvimrc and .exrc in the current directory
        fileencoding | fenc : String => "", // file encoding for multibyte text

        fileformat | ff : String => "unix", // file format used for file I/O
        fileformats | ffs : String => "unix", // automatically detected values for 'fileformat'

        filetype | ft : String => "", // type of file, used for autocommands
        fixendofline | fixeol : bool => "true", // make sure last line in file has <EOL>
        foldtext | fdt : String => "foldtext()", // expression used to display for a closed fold
        formatlistpat | flp : String => "^\\s*\\d\\+[\\]:.)}\\t ]\\s*", // pattern used to recognize a list header
        formatoptions | fo : String => "tcqj", // how automatic formatting is to be done
        formatprg | fp : String => "", // name of external program used with "gq" command
        grepprg | gp : String => "grep -n ", // program to use for ":grep"
    }
}

options! {
    WinOptions {
        arabic | arab : bool => "false", // for Arabic as a default second language
        breakindent | bri : bool => "false", // wrapped line repeats indent
        breakindentopt | briopt : bool => "false", // settings for 'breakindent'
        colorcolumn | cc : String => "", // columns to highlight

        concealcursor | cocu : String => "", // whether concealable text is hidden in cursor line
        conceallevel | cole : isize => "0", // whether concealable text is shown or hidden

        cursorbind | crb : bool => "false", // move cursor in window as it moves in other windows
        cursorcolumn | cuc : bool => "false", // highlight the screen column of the cursor
        cursorline | cul : bool => "false", // highlight the screen line of the cursor
        cursorlineopt | culopt : String => "both", // settings for 'cursorline'
        diff : bool => "false", // use diff mode for the current window
        fillchars | fcs : String => "", // characters to use for displaying special items

        foldcolumn | fdc : isize => "0", // width of the column used to indicate folds
        foldenable | fen : bool => "true", // set to display all folds open
        foldexpr | fde : String => "0", // expression used when 'foldmethod' is "expr"
        foldignore | fdi : String => "#", // ignore lines when 'foldmethod' is "indent"
        foldlevel | fdl : isize => "0", // close folds with a level higher than this

        foldmarker | fmr : String => "{{{,}}}", // markers used when 'foldmethod' is "marker"
        foldmethod | fdm : String => "manual", // folding type
        foldminlines | fml : isize => "1", // minimum number of lines for a fold to be closed
        foldnestmax | fdn : isize => "20", // maximum fold depth
        foldopen | fdo : String => "block,hor,mark,percent,quickfix,search,tag,undo", // for which commands a fold will be opened
    }
}
