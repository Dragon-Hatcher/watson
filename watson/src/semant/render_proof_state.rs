use super::fragments::FragData;
use crate::semant::{
    check_proofs::ProofState,
    formal_syntax::{FormalSyntax, FormalSyntaxPatternPart},
    fragments::{FragCtx, FragId, FragPart},
    theorem::{Template, TheoremStatement},
};
use crate::util::ansi::*;
use itertools::Itertools;

pub fn render_proof_state(
    state: &ProofState,
    in_theorem: &TheoremStatement,
    ctx: &FragCtx,
    formal: &FormalSyntax,
) -> String {
    let mut result = String::new();

    for (cat, template_chunk) in in_theorem
        .templates()
        .iter()
        .chunk_by(|t| t.cat())
        .into_iter()
    {
        result.push_str("   ");
        for template in template_chunk {
            let template_str = format_template(template);
            result.push_str(&format!(" {template_str}"));
        }
        result.push_str(&format!(" : {}\n", cat.name()));
    }

    let mut shorthands = state.shorthands().keys().copied().collect_vec();
    shorthands.sort_by_key(|s| s.as_str());

    for shorthand_name in shorthands {
        let shorthand_for = state.shorthands()[&shorthand_name];
        result.push_str(&format!(
            "    {ANSI_YELLOW}{ANSI_BOLD}{shorthand_name}{ANSI_RESET} := {}\n",
            format_frag(shorthand_for, ctx, formal, &mut Vec::new())
        ));
    }

    let mut knowns = state.knowns().iter().copied().collect_vec();
    knowns.sort();

    for fact in knowns {
        if let Some(assumption) = fact.assumption() {
            result.push_str(&format!(
                "    {} |- {}\n",
                format_frag(assumption, ctx, formal, &mut Vec::new()),
                format_frag(fact.sentence(), ctx, formal, &mut Vec::new())
            ));
        } else {
            result.push_str(&format!(
                "    {}\n",
                format_frag(fact.sentence(), ctx, formal, &mut Vec::new())
            ));
        }
    }

    result.push_str(&format!(
        "    {ANSI_BLUE}{ANSI_BOLD}|-{ANSI_RESET} {}",
        format_frag(state.goal(), ctx, formal, &mut Vec::new())
    ));

    result
}

fn format_template(template: &Template) -> String {
    let mut result = String::new();

    result.push_str(ANSI_YELLOW);
    result.push_str(ANSI_BOLD);
    result.push_str(&template.name());
    result.push_str(ANSI_RESET);
    if !template.params().is_empty() {
        result.push('(');
        for (i, param) in template.params().iter().enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&param.name());
        }
        result.push(')');
    }

    result
}

fn format_frag(
    frag: FragId,
    ctx: &FragCtx,
    formal: &FormalSyntax,
    bindings: &mut Vec<String>,
) -> String {
    let frag = ctx.get(frag);

    match frag.data() {
        FragData::Rule {
            rule,
            bindings: bindings_count,
            parts,
        } => {
            let mut result = String::new();

            for _ in 0..*bindings_count {
                bindings.push(format!("v{}", bindings.len()));
            }

            let rule_pat = formal.get_rule(*rule);
            let mut frag_part_idx = 0;
            let mut binding_idx = 0;
            for part in rule_pat.pat().parts() {
                match part {
                    FormalSyntaxPatternPart::Lit(lit) => {
                        if !result.is_empty() {
                            result.push(' ');
                        }
                        result.push_str(lit);
                    }
                    FormalSyntaxPatternPart::Binding(_) => {
                        if !result.is_empty() {
                            result.push(' ');
                        }
                        result.push_str(&format!(
                            "v{}",
                            bindings.len() - bindings_count + binding_idx
                        ));
                        binding_idx += 1;
                    }
                    FormalSyntaxPatternPart::Variable(_) | FormalSyntaxPatternPart::Cat(_) => {
                        let frag_part = &parts[frag_part_idx];
                        frag_part_idx += 1;
                        let s = match frag_part {
                            FragPart::Var(idx) => bindings[bindings.len() - 1 - idx].clone(),
                            FragPart::Frag(frag_id) => format_frag(*frag_id, ctx, formal, bindings),
                        };
                        if !result.is_empty() {
                            result.push(' ');
                        }
                        result.push_str(&s);
                    }
                }
            }

            result
        }
        FragData::Template { name, args } => {
            if args.is_empty() {
                format!("{name}")
            } else {
                let args_str = args
                    .iter()
                    .map(|arg| format_frag(*arg, ctx, formal, bindings))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({args_str})")
            }
        }
        FragData::TemplateArgHole(idx) => format!("_{idx}"),
    }
}
