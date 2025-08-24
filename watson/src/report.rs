use crate::util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RED, ANSI_RESET, ANSI_YELLOW};
use crate::{semant::ProofReport, util::plural};

pub fn display_report(proof_report: &ProofReport) -> bool {
    let mut theorems_used = 0;
    let mut axioms_used = 0;

    let mut correct_count = 0;
    let mut todo_count = 0;
    let mut error_count = 0;

    for status in proof_report.statuses.values() {
        if status.is_axiom() {
            axioms_used += 1;
        } else {
            theorems_used += 1;
        }

        if status.proof_correct() {
            if status.todo_used() {
                todo_count += 1;
            } else {
                correct_count += 1;
            }
        } else {
            error_count += 1;
        }
    }

    println!(
        "Checked {} theorem{} ({} axiom{}, {} theorem{}):",
        proof_report.statuses.len(),
        plural(proof_report.statuses.len()),
        axioms_used,
        plural(axioms_used),
        theorems_used,
        plural(theorems_used)
    );

    println!(
        " {ANSI_GREEN}✓{ANSI_RESET} {ANSI_BOLD}{correct_count}{ANSI_RESET} theorem{} correct. ",
        plural(correct_count)
    );
    if todo_count > 0 {
        println!(
            " {ANSI_YELLOW}✓{ANSI_RESET} {ANSI_BOLD}{todo_count}{ANSI_RESET} theorem{} with todo.",
            plural(todo_count)
        );
    }
    if error_count > 0 {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{error_count}{ANSI_RESET} theorem{} with errors.",
            plural(error_count)
        );
    }

    if !proof_report.circular_groups.is_empty() {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} circular dependency group{} detected.",
            proof_report.circular_groups.len(),
            plural(proof_report.circular_groups.len())
        );

        for group in &proof_report.circular_groups {
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

    let all_ok = error_count == 0 && proof_report.circular_groups.is_empty();

    if all_ok {
        println!();
        println!("🎉 All theorems correct! 🎉");
    }

    all_ok
}
