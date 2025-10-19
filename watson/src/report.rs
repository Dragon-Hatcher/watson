use crate::semant::proof_status::ProofStatuses;
use crate::semant::theorems::TheoremId;
use crate::util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_GRAY, ANSI_RED, ANSI_RESET, ANSI_YELLOW};
use crate::util::plural;

pub struct ProofReport<'ctx> {
    pub statuses: ProofStatuses<'ctx>,
    pub circularities: Vec<Vec<TheoremId<'ctx>>>,
}

pub fn display_report(report: &ProofReport, iteration: Option<usize>) -> bool {
    let ProofReport {
        statuses,
        circularities,
    } = report;

    let iter_info = match iteration {
        Some(iter) => format!("{ANSI_GRAY}iteration {iter}{ANSI_RESET}"),
        None => String::new(),
    };

    println!(
        "Checked {} theorem{} ({} axiom{}, {} theorem{}): {}",
        statuses.total_cnt(),
        plural(statuses.total_cnt()),
        statuses.axiom_cnt(),
        plural(statuses.axiom_cnt()),
        statuses.theorem_cnt(),
        plural(statuses.theorem_cnt()),
        iter_info
    );

    println!(
        " {ANSI_GREEN}✓{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} correct. ",
        statuses.correct_cnt(),
        plural(statuses.correct_cnt())
    );
    if statuses.todo_cnt() > 0 {
        println!(
            " {ANSI_YELLOW}~{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} with todo.",
            statuses.todo_cnt(),
            plural(statuses.todo_cnt())
        );
    }
    if statuses.error_cnt() > 0 {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} with errors.",
            statuses.error_cnt(),
            plural(statuses.error_cnt())
        );
    }

    if !circularities.is_empty() {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} circular dependency group{} detected.",
            circularities.len(),
            plural(circularities.len())
        );

        for group in circularities {
            print!("     -");
            for (i, thm) in group.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                print!(" {}", thm.name());
            }
            println!();
        }
    }

    let all_ok = statuses.error_cnt() == 0 && circularities.is_empty();

    if all_ok {
        println!();
        println!("🎉 All theorems correct! 🎉");
    }

    all_ok
}
