pub type WResult<T> = Result<T, ()>;

pub struct DiagManager {
    diags: Vec<Diagnostic>,
}

impl DiagManager {
    pub fn new() -> Self {
        Self { diags: Vec::new() }
    }

    pub fn print_errors(&self) {
        for diag in &self.diags {
            println!("Error: {}", diag.msg);
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }
}

struct Diagnostic {
    msg: &'static str,
}

impl DiagManager {
    fn add_diag(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }
}

impl DiagManager {
    pub fn err_source_redeclaration<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_source_redeclaration",
        });

        Err(())
    }

    pub fn err_non_existent_file<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_non_existent_file",
        });

        Err(())
    }

    pub fn err_elaboration_infinite_recursion<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_elaboration_infinite_recursion",
        });

        Err(())
    }

    pub fn err_parse_failure<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_parse_failure",
        });

        Err(())
    }

    pub fn err_duplicate_formal_syntax_cat<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_duplicate_formal_syntax_cat",
        });

        Err(())
    }

    pub fn err_duplicate_formal_syntax_rule<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_duplicate_formal_syntax_rule",
        });

        Err(())
    }

    pub fn err_unknown_formal_syntax_cat<T>(&mut self) -> WResult<T> {
        self.add_diag(Diagnostic {
            msg: "err_unknown_formal_syntax_cat",
        });

        Err(())
    }
}
