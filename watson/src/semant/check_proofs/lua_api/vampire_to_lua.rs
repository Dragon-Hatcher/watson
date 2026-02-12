use std::time::Duration;

use mlua::{FromLua, MetaMethod, UserData, Variadic};
use vampire_prover::{Formula, Function, Options, Predicate, Problem, ProofRes, Term};

#[derive(Debug, Clone, FromLua)]
pub struct LuaVFunction {
    function: Function,
}

impl UserData for LuaVFunction {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("with", |_, this, args: Vec<LuaVTerm>| {
            let terms: Vec<Term> = args.into_iter().map(|t| t.term).collect();
            let result_term = this.function.with(&terms);
            Ok(LuaVTerm { term: result_term })
        });
    }
}

pub struct LuaVFunctionMeta;

impl UserData for LuaVFunctionMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, (name, arity): (String, u32)| {
            let function = Function::new(&name, arity);
            Ok(LuaVFunction { function })
        });
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct LuaVPredicate {
    predicate: Predicate,
}

impl UserData for LuaVPredicate {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("with", |_, this, args: Vec<LuaVTerm>| {
            let terms: Vec<Term> = args.into_iter().map(|t| t.term).collect();
            let result_formula = this.predicate.with(&terms);
            Ok(LuaVFormula {
                formula: result_formula,
            })
        });
    }
}

pub struct LuaVPredicateMeta;

impl UserData for LuaVPredicateMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, (name, arity): (String, u32)| {
            let predicate = Predicate::new(&name, arity);
            Ok(LuaVPredicate { predicate })
        });
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct LuaVTerm {
    term: Term,
}

impl UserData for LuaVTerm {}

pub struct LuaVTermMeta;

impl UserData for LuaVTermMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("newVar", |_, _, idx: u32| {
            let term = Term::new_var(idx);
            Ok(LuaVTerm { term })
        });
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct LuaVFormula {
    formula: Formula,
}

impl UserData for LuaVFormula {}

pub struct LuaVFormulaMeta;

impl UserData for LuaVFormulaMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("newEq", |_, _, (t1, t2): (LuaVTerm, LuaVTerm)| {
            let formula = t1.term.eq(t2.term);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newAnd", |_, _, formulas: Variadic<LuaVFormula>| {
            let formulas: Vec<Formula> = formulas.into_iter().map(|f| f.formula).collect();
            let formula = Formula::new_and(&formulas);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newOr", |_, _, formulas: Variadic<LuaVFormula>| {
            let formulas: Vec<Formula> = formulas.into_iter().map(|f| f.formula).collect();
            let formula = Formula::new_or(&formulas);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newNot", |_, _, f: LuaVFormula| {
            let formula = !f.formula;
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newImp", |_, _, (ant, cons): (LuaVFormula, LuaVFormula)| {
            let formula = ant.formula >> cons.formula;
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newIff", |_, _, (f1, f2): (LuaVFormula, LuaVFormula)| {
            let formula = f1.formula.iff(f2.formula);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newForall", |_, _, (var, f): (u32, LuaVFormula)| {
            let formula = Formula::new_forall(var, f.formula);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newExists", |_, _, (var, f): (u32, LuaVFormula)| {
            let formula = Formula::new_exists(var, f.formula);
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newTrue", |_, _, _: ()| {
            let formula = Formula::new_true();
            Ok(LuaVFormula { formula })
        });

        methods.add_method("newFalse", |_, _, _: ()| {
            let formula = Formula::new_false();
            Ok(LuaVFormula { formula })
        });
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct LuaVOptions {
    options: Options,
}

impl UserData for LuaVOptions {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("setTimeout", |_, this, timeout_ms: u64| {
            this.options.timeout(Duration::from_millis(timeout_ms));
            Ok(())
        });
    }
}

pub struct LuaVOptionsMeta;

impl UserData for LuaVOptionsMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, _: ()| {
            let options = Options::new();
            Ok(LuaVOptions { options })
        });
    }
}

// ============================================================================
// VProblem - Wrapper for vampire_prover::Problem
// ============================================================================

#[derive(Debug, Clone, FromLua)]
pub struct LuaVProblem {
    problem: Problem,
}

impl UserData for LuaVProblem {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("addAxiom", |_, this, axiom: LuaVFormula| {
            this.problem.with_axiom(axiom.formula);
            Ok(())
        });

        methods.add_method_mut("setConjecture", |_, this, conj: LuaVFormula| {
            this.problem.conjecture(conj.formula);
            Ok(())
        });

        methods.add_method_mut("solve", |_, this, _: ()| {
            let result = this.problem.solve();
            let result_str = match result {
                ProofRes::Proved => "proved",
                ProofRes::Unprovable => "unprovable",
                ProofRes::Unknown(_) => "unknown",
            };
            Ok(result_str.to_string())
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{:#?}", this.problem))
        });
    }
}

pub struct LuaVProblemMeta;

impl UserData for LuaVProblemMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, options: LuaVOptions| {
            let problem = Problem::new(options.options);
            Ok(LuaVProblem { problem })
        });
    }
}
