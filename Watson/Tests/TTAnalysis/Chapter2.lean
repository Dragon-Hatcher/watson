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
  exact Nat.succ_ne_zero

theorem prop_2_1_6 : (4 : Nat) ≠ 0 := by
  exact Nat.succ_ne_zero

theorem axiom_2_4 {n m : Nat} : n ≠ m → n.succ ≠ m.succ := by
  contrapose!
  exact Nat.succ_inj.mp

theorem prop_2_1_8 : (6 : Nat) ≠ 2 := by
  apply axiom_2_4
  apply axiom_2_4
  exact Nat.succ_ne_zero

theorem axiom_2_5 {motive : Nat → Prop}
  (zero : motive Nat.zero) (succ : (n : Nat) → motive n → motive n.succ)
  : ∀ n, motive n := by
  intro n
  exact Nat.rec zero succ n


/-! # Section 2.2 -/

variable (a b c n m : Nat)

theorem lemma_2_2_2 : n + 0 = n := Nat.add_zero n

theorem lemma_2_2_3 : n.succ + m = (n + m).succ := Nat.succ_add n m

theorem prop_2_2_4 : a + b = b + a := Nat.add_comm a b

theorem prop_2_2_5 : (a + b) + c = a + (b + c) :=  Nat.add_assoc a b c

theorem prop_2_2_6 : a + b = a + c → b = c := Nat.add_cancels

theorem prop_2_2_8 : a.is_pos → (a + b).is_pos := Nat.pos_add_is_pos a b

theorem lemma_2_2_10 : a.is_pos → ∃ b : Nat, b.succ = a := Nat.exists_pred a

theorem prop_2_2_12_a : a ≤ a := Nat.le_rfl a
theorem prop_2_2_12_b : a ≤ b → b ≤ c → a ≤ c := Nat.le_trans
theorem prop_2_2_12_c : a ≤ b → b ≤ a → a = b := Nat.le_anti_symm
theorem prop_2_2_12_d : a ≤ b ↔ a + c ≤ b + c := Nat.le_iff_le_add
theorem prop_2_2_12_e : a < b ↔ a.succ ≤ b := Nat.lt_iff_succ_le
theorem prop_2_2_12_f : a < b ↔ ∃ c, b = a + c ∧ c.is_pos := Eq.to_iff rfl

theorem prop_2_2_13 : a < b ∨ a = b ∨ a > b := Nat.lt_trichotomy a b

theorem prop_2_2_14 (motive : Nat → Prop) (m₀ : Nat)
  (h : ∀ m ≥ m₀, (∀ m', m₀ ≤ m' ∧ m' < m → motive m') → motive m)
  : ∀ m ≥ m₀, motive m := Nat.strong_induction' motive m₀ h

theorem ex_2_2_6 (motive : Nat → Prop) (n : Nat)
  (h: ∀ m, motive m.succ → motive m) (hn : motive n)
  : ∀ m ≤ n, motive m := by
  induction n with
  | zero       =>
      intro m m_le_zero
      rw [Nat.le_zero_eq_zero m m_le_zero]
      assumption
  | succ n' ih =>
      intro m m_le_n'_succ
      have hn' : motive n' := h n' hn
      cases Nat.le_lt_or_eq m_le_n'_succ with
      | inr m_eq_n'      => rw [m_eq_n']; assumption
      | inl m_lt_n'_succ =>
          have m_le_n' : m ≤ n' := Nat.lt_succ_iff_le.mp m_lt_n'_succ
          exact ih hn' m m_le_n'

end Watson
