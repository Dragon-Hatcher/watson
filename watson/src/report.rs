use crate::semant::ProofReport;

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_BOLD: &str = "\x1b[1m";

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
        "Checked {} theorems ({} axioms, {} theorems):",
        proof_report.statuses.len(),
        axioms_used,
        theorems_used
    );

    println!(
        " {ANSI_GREEN}✓{ANSI_RESET} {ANSI_BOLD}{correct_count}{ANSI_RESET} theorems correct. "
    );
    if todo_count > 0 {
        println!(
            " {ANSI_YELLOW}✓{ANSI_RESET} {ANSI_BOLD}{todo_count}{ANSI_RESET} theorems with todo."
        );
    }
    if error_count > 0 {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{error_count}{ANSI_RESET} theorems with errors."
        );
    }

    if !proof_report.circular_groups.is_empty() {
        let s = if proof_report.circular_groups.len() == 1 {
            ""
        } else {
            "s"
        };
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} circular dependency group{s} detected.",
            proof_report.circular_groups.len(),
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
