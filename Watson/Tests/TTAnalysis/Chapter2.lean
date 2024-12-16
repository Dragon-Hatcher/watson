import Watson.Nat
import Mathlib

namespace Watson

/-! # Section 2.1 -/

theorem axiom_2_1 : ∃ n : Nat, n = 0 := by
  use Nat.zero
  exact Nat.zero_eq_lit

theorem axiom_2_2 : ∀ n : Nat, ∃ m : Nat, m = n.succ := by
  intro n
  use n.succ

theorem prop_2_1_4 : ∃ n : Nat, n = 3 := by
  use Nat.zero.succ.succ.succ
  rfl

theorem axiom_2_3 : ∀ n : Nat, n.succ ≠ 0 := by
  intro n
  exact Nat.succ_neq_zero

theorem prop_2_1_6 : (4 : Nat) ≠ 0 := by
  exact Nat.succ_neq_zero

theorem axiom_2_4 {n m : Nat} : n ≠ m → n.succ ≠ m.succ := by
  contrapose!
  exact Nat.succ_inj.mp

theorem prop_2_1_8 : (6 : Nat) ≠ 2 := by
  apply axiom_2_4
  apply axiom_2_4
  exact Nat.succ_neq_zero

theorem axiom_2_5 {motive : Nat → Prop}
  (zero : motive Nat.zero) (succ : (n : Nat) → motive n → motive n.succ)
  : ∀ n, motive n := by
  intro n
  exact Nat.rec zero succ n


/-! # Section 2.2 -/

end Watson
