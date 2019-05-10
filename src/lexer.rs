use regex::Regex;

use crate::token::{Error, Token};

use lazy_static::lazy_static;

pub type Spanned<T> = ((usize, usize), T, (usize, usize));

lazy_static! {
    static ref PERIOD        : Regex = Regex::new(r"^\.").unwrap();
    static ref COMMA         : Regex = Regex::new(r"^,").unwrap();
    static ref SEMICOLON     : Regex = Regex::new(r"^;").unwrap();
    static ref DOTDOT        : Regex = Regex::new(r"^\.\.").unwrap();
    static ref PROGRAM       : Regex = Regex::new(r"^PROGRAM").unwrap();
    static ref LABEL         : Regex = Regex::new(r"^LABEL").unwrap();
    static ref CONST         : Regex = Regex::new(r"^CONST").unwrap();
    static ref EQUALS        : Regex = Regex::new(r"^=").unwrap();
    static ref POW           : Regex = Regex::new(r"^\*\*").unwrap();
    static ref LPAREN        : Regex = Regex::new(r"^\(").unwrap();
    static ref RPAREN        : Regex = Regex::new(r"^\)").unwrap();
    static ref NOT           : Regex = Regex::new(r"^NOT").unwrap();
    static ref PLUS          : Regex = Regex::new(r"^\+").unwrap();
    static ref MINUS         : Regex = Regex::new(r"^-").unwrap();
    static ref TYPE          : Regex = Regex::new(r"^TYPE").unwrap();
    static ref ARRAY         : Regex = Regex::new(r"^ARRAY").unwrap();
    static ref LBRACKET      : Regex = Regex::new(r"^\[").unwrap();
    static ref RBRACKET      : Regex = Regex::new(r"^\]").unwrap();
    static ref OF            : Regex = Regex::new(r"^OF").unwrap();
    static ref RECORD        : Regex = Regex::new(r"^RECORD").unwrap();
    static ref BEGIN         : Regex = Regex::new(r"^BEGIN").unwrap();
    static ref END           : Regex = Regex::new(r"^END").unwrap();
    static ref COLON         : Regex = Regex::new(r"^:").unwrap();
    static ref CASE          : Regex = Regex::new(r"^CASE").unwrap();
    static ref SET           : Regex = Regex::new(r"^SET").unwrap();
    static ref PFILE         : Regex = Regex::new(r"^PFILE").unwrap();
    static ref POINTER       : Regex = Regex::new(r"^\^").unwrap();
    static ref VAR           : Regex = Regex::new(r"^VAR").unwrap();
    static ref FORWARD       : Regex = Regex::new(r"^FORWARD").unwrap();
    static ref EXTERNAL      : Regex = Regex::new(r"^EXTERNAL").unwrap();
    static ref PROCEDURE     : Regex = Regex::new(r"^PROCEDURE").unwrap();
    static ref FUNCTION      : Regex = Regex::new(r"^FUNCTION").unwrap();
    static ref REPEAT        : Regex = Regex::new(r"^REPEAT").unwrap();
    static ref UNTIL         : Regex = Regex::new(r"^UNTIL").unwrap();
    static ref WHILE         : Regex = Regex::new(r"^WHILE").unwrap();
    static ref DO            : Regex = Regex::new(r"^DO").unwrap();
    static ref FOR           : Regex = Regex::new(r"^FOR").unwrap();
    static ref ASSIGN        : Regex = Regex::new(r"^:=").unwrap();
    static ref WITH          : Regex = Regex::new(r"^WITH").unwrap();
    static ref IF            : Regex = Regex::new(r"^IF").unwrap();
    static ref THEN          : Regex = Regex::new(r"^THEN").unwrap();
    static ref ELSE          : Regex = Regex::new(r"^ELSE").unwrap();
    static ref GOTO          : Regex = Regex::new(r"^GOTO").unwrap();
    static ref OTHERWISE     : Regex = Regex::new(r"^OTHERWISE").unwrap();
    static ref TO            : Regex = Regex::new(r"^TO").unwrap();
    static ref DOWNTO        : Regex = Regex::new(r"^DOWNTO").unwrap();
    static ref NIL           : Regex = Regex::new(r"^NIL").unwrap();
    static ref OR            : Regex = Regex::new(r"^OR").unwrap();
    static ref MUL           : Regex = Regex::new(r"^\*").unwrap();
    static ref FDIV          : Regex = Regex::new(r"^/").unwrap();
    static ref IDIV          : Regex = Regex::new(r"^DIV").unwrap();
    static ref MOD           : Regex = Regex::new(r"^MOD").unwrap();
    static ref AND           : Regex = Regex::new(r"^AND").unwrap();
    static ref NOTEQUALS     : Regex = Regex::new(r"^<>").unwrap();
    static ref LESS          : Regex = Regex::new(r"^<").unwrap();
    static ref GREATER       : Regex = Regex::new(r"^>").unwrap();
    static ref LESSEQUALS    : Regex = Regex::new(r"^<=").unwrap();
    static ref GREATEREQUALS : Regex = Regex::new(r"^>=").unwrap();
    static ref IN            : Regex = Regex::new(r"^IN").unwrap();

    static ref NEWLINE : Regex = Regex::new(r"^\n").unwrap();

    static ref IDENTIFIER : Regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9]*").unwrap();
    static ref STRING     : Regex = Regex::new(r"^'[^']*'").unwrap();
    static ref FLOAT      : Regex = Regex::new(r"^\d+\.\d+").unwrap();
    static ref INTEGER    : Regex = Regex::new(r"^\d+").unwrap();
    //
    static ref COMMENT          : Regex = Regex::new(r"^\{.*?}").unwrap();
    static ref MULTILINECOMMENT : Regex = Regex::new(r"^\(\*(.|\n)*?\*\)").unwrap();
    static ref WS               : Regex = Regex::new(r"^[[:blank:]]").unwrap();
}

pub struct Lexer<'input> {
    text: &'input str,
    pub pos: usize,
    start: usize,
    srow: usize,
    scol: usize,
    erow: usize,
    ecol: usize,
}

impl<'input> Lexer<'input> {
    pub fn new(text: &'input str) -> Lexer<'input> {
        Lexer {
            text: text,
            pos: 0,
            start: 0,
            srow: 1,
            scol: 1,
            erow: 1,
            ecol: 1,
        }
    }

    fn match_and_consume<F>(
        text: &mut &'input str,
        pos: &mut usize,
        start: &mut usize,
        scol: &mut usize,
        ecol: &mut usize,
        re: &Regex,
        action: F,
    ) -> Option<Token>
    where
        F: Fn(&'input str) -> Token,
    {
        if let Some(mat) = re.find(text) {
            *start = *pos;
            *pos += mat.end();
            let ret = Some(action(&text[mat.start()..mat.end()]));
            *text = &text[mat.end()..];
            *scol = *ecol;
            *ecol += mat.end();
            ret
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        loop {
            if let Some(mat) = COMMENT.find(self.text) {
                self.start = self.pos;
                self.pos += mat.end();
                self.text = &self.text[mat.end()..];
                self.srow += 1;
                self.scol = 1;
                self.erow += 1;
                self.ecol = 1;
                continue;
            } else if let Some(mat) = MULTILINECOMMENT.find(self.text) {
                // think what to do with row and col
                self.start = self.pos;
                self.pos += mat.end();
                self.text = &self.text[mat.end()..];
                continue;
            } else if let Some(mat) = NEWLINE.find(self.text) {
                self.start = self.pos;
                self.pos += mat.end();
                self.text = &self.text[mat.end()..];
                self.srow += 1;
                self.scol = 1;
                self.erow += 1;
                self.ecol = 1;
                continue;
            } else if let Some(mat) = WS.find(self.text) {
                self.start = self.pos;
                self.pos += mat.end();
                self.text = &self.text[mat.end()..];
                self.scol = self.ecol;
                self.ecol += mat.end();
                continue;
            }
            break;
        }

        macro_rules! actions {
            ( $( $x:expr => $y:expr ),* ) => {
                if false { None } /* 'if false' just to make the rust syntax below more uniform */
                $(
                    else if let t@Some(_) = Lexer::match_and_consume(&mut self.text,
                                                                     &mut self.pos,
                                                                     &mut self.start,
                                                                     &mut self.scol,
                                                                     &mut self.ecol,
                                                                     &$x,
                                                                     $y) { t }
                )*
                else { None }
            };
        }

        let result = actions![
            FLOAT   => |s: &'input str| Token::Float(s.parse().unwrap()),
            INTEGER => |s: &'input str| Token::Integer(s.parse().unwrap()),

            OTHERWISE     => |_| Token::Otherwise,
            COMMA         => |_| Token::Comma,
            SEMICOLON     => |_| Token::Semicolon,
            ASSIGN        => |_| Token::Assign,
            DOTDOT        => |_| Token::Dotdot,
            PROCEDURE     => |_| Token::Procedure,
            PROGRAM       => |_| Token::Program,
            LABEL         => |_| Token::Label,
            CONST         => |_| Token::Const,
            EQUALS        => |_| Token::Equals,
            POW           => |_| Token::Pow,
            LPAREN        => |_| Token::LParen,
            RPAREN        => |_| Token::RParen,
            NOT           => |_| Token::Not,
            PLUS          => |_| Token::Plus,
            MINUS         => |_| Token::Minus,
            TYPE          => |_| Token::Type,
            ARRAY         => |_| Token::Array,
            LBRACKET      => |_| Token::LBracket,
            RBRACKET      => |_| Token::RBracket,
            OF            => |_| Token::Of,
            RECORD        => |_| Token::Record,
            BEGIN         => |_| Token::Begin,
            END           => |_| Token::End,
            COLON         => |_| Token::Colon,
            CASE          => |_| Token::Case,
            SET           => |_| Token::Set,
            PFILE         => |_| Token::PFile,
            POINTER       => |_| Token::Pointer,
            VAR           => |_| Token::Var,
            FORWARD       => |_| Token::Forward,
            EXTERNAL      => |_| Token::External,
            FUNCTION      => |_| Token::Function,
            REPEAT        => |_| Token::Repeat,
            UNTIL         => |_| Token::Until,
            WHILE         => |_| Token::While,
            DOWNTO        => |_| Token::Downto,
            DO            => |_| Token::Do,
            FOR           => |_| Token::For,
            WITH          => |_| Token::With,
            IF            => |_| Token::If,
            THEN          => |_| Token::Then,
            ELSE          => |_| Token::Else,
            GOTO          => |_| Token::Goto,
            TO            => |_| Token::To,
            NIL           => |_| Token::Nil,
            OR            => |_| Token::Or,
            MUL           => |_| Token::Mul,
            FDIV          => |_| Token::FDiv,
            IDIV          => |_| Token::IDiv,
            MOD           => |_| Token::Mod,
            AND           => |_| Token::And,
            NOTEQUALS     => |_| Token::NotEquals,
            LESSEQUALS    => |_| Token::LessEquals,
            GREATEREQUALS => |_| Token::GreaterEquals,
            LESS          => |_| Token::Less,
            GREATER       => |_| Token::Greater,
            IN            => |_| Token::In,
            PERIOD        => |_| Token::Period,

            STRING     => |s: &'input str| Token::String(s[1..s.len()-1].to_string()),
            IDENTIFIER => |s: &'input str| Token::Identifier(s.to_string())
        ];

        result
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<Spanned<Token>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Some(t) => Some(Ok(((self.srow, self.scol), t, (self.erow, self.ecol)))),
            None => None,
        }
    }
}
