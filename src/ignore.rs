use std::fmt::Debug;

pub struct FileMatchExpr(Vec<Piece<Vec<Piece<char>>>>);
pub struct StrToMatch(Vec<Vec<char>>);

impl FileMatchExpr {
    pub fn compile_target(targ: &str) -> StrToMatch {
        StrToMatch(targ.split('/').map(|s| s.chars().collect()).collect())
    }

    pub fn compile(mut line: &str) -> FileMatchExpr {
        let mut expr = vec![];
        let mut dir = false;

        if line.starts_with('/') {
            line = &line[1..];
        } else {
            expr.push(Piece::Any);
        }

        if line.ends_with('/') {
            line = &line[..line.len() - 1];
            dir = true;
        }

        expr.extend(line.split('/').map(|p| {
            match p {
                "**" => Piece::Any,
                s => Piece::Piece(
                    s.chars()
                        .map(|c| match c {
                            '*' => Piece::Any,
                            c => Piece::Piece(c),
                        })
                        .collect(),
                ),
            }
        }));

        if dir {
            expr.push(Piece::Any);
        }
        FileMatchExpr(expr)
    }

    pub fn match_file(&self, file: &str) -> bool {
        self.0
            .match_expr(&file.split('/').map(|s| s.chars().collect()).collect())
    }

    pub fn match_file_compiled(&self, targ: &StrToMatch) -> bool {
        self.0.match_expr(&targ.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Piece<T: Debug + Eq + PartialEq> {
    Any,
    Piece(T),
}

pub trait Match {
    type MatchType;
    fn match_expr(&self, targ: &Self::MatchType) -> bool;
}

impl Match for char {
    type MatchType = char;
    fn match_expr(&self, targ: &Self::MatchType) -> bool {
        self == targ || *self == '?'
    }
}

impl<S: Match + Debug + Eq + PartialEq> Match for Vec<Piece<S>> {
    type MatchType = Vec<S::MatchType>;
    fn match_expr(&self, targ: &Vec<S::MatchType>) -> bool {
        let n = targ.len();
        let m = self.len();
        let mut results = vec![vec![None; m + 1]; n + 1];

        fn match_body<T: Match + Debug + Eq + PartialEq>(
            targ: &[T::MatchType],
            expr: &[Piece<T>],
            i_targ: usize,
            i_expr: usize,
            result_table: &mut Vec<Vec<Option<bool>>>,
        ) -> bool {
            if let Some(res) = result_table[i_targ][i_expr] {
                return res;
            }

            let result = if i_expr == expr.len() {
                i_targ == targ.len()
            } else {
                match &expr[i_expr] {
                    Piece::Piece(p) => {
                        i_targ != targ.len()
                            && p.match_expr(&targ[i_targ])
                            && match_body(targ, expr, i_targ + 1, i_expr + 1, result_table)
                    }
                    Piece::Any => {
                        (i_targ != targ.len()
                            && match_body(targ, expr, i_targ + 1, i_expr, result_table))
                            || match_body(targ, expr, i_targ, i_expr + 1, result_table)
                    }
                }
            };

            result_table[i_targ][i_expr] = Some(result);
            result
        }

        match_body(targ, &self, 0, 0, &mut results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_test() {
        fn match_string_wildcard(expr: &str, targ: &str) -> bool {
            let exprs: Vec<_> = expr
                .chars()
                .map(|c| match c {
                    '*' => Piece::Any,
                    c => Piece::Piece(c),
                })
                .collect();

            exprs.match_expr(&targ.chars().collect::<Vec<_>>())
        }

        assert_eq!(match_string_wildcard("d*", "bcc"), false);
        assert_eq!(match_string_wildcard("d*a", "dababababa"), true);
        assert_eq!(match_string_wildcard("d*e", "d"), false);
    }

    #[test]
    fn match_file_test() {
        let pat = FileMatchExpr::compile("hoge/fuga.rs");
        assert_eq!(pat.match_file("hoge.rs"), false);
        assert_eq!(pat.match_file("hoge/fuga.rs"), true);
        assert_eq!(pat.match_file("piyo/hoge/fuga.rs"), true);
        assert_eq!(pat.match_file("hoge/fuga/piyo.rs"), false);

        let pat = FileMatchExpr::compile("/**/*.r?");
        assert_eq!(pat.match_file("hoge.rs"), true);
        assert_eq!(pat.match_file("hoge.hs"), false);
        assert_eq!(pat.match_file("hoge.ro"), true);
        assert_eq!(pat.match_file("some/nested/dir/fuga.rs"), true);

        let pat = FileMatchExpr::compile("hoge/**/fuga/*.rs");
        assert_eq!(pat.match_file("hoge/fuga/piyo.rs"), true);
        assert_eq!(pat.match_file("hoge.hs"), false);
        assert_eq!(pat.match_file("some/nested/dir/fuga.rs"), false);
        assert_eq!(pat.match_file("hoge/some/nested/fuga/piyo.rs"), true);

        let pat = FileMatchExpr::compile("hoge/");
        assert_eq!(pat.match_file("hoge/fuga/piyo.rs"), true);
        assert_eq!(pat.match_file("hoge.hs"), false);
        assert_eq!(pat.match_file("some/nested/dir/hoge/fuga.rs"), true);
        assert_eq!(pat.match_file("hoge/some/nested/piyo.rs"), true);
    }
}
