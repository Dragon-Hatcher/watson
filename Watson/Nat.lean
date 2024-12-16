import Mathlib

namespace Watson

open _root_ renaming Nat → LeanNat

/--
The integers greater than or equal to zero. Defined in terms of zero and a
successor function.
-/
inductive Nat where
  | zero : Nat
  | succ (n : Nat) : Nat


/-! Test -/
-- Here we make it so we can use integer literals for instances of our custom
-- `Nat` type.

def ofNat : (n : LeanNat) → Nat
  | LeanNat.zero   => Nat.zero
  | LeanNat.succ m => Nat.succ (ofNat m)

instance (n : LeanNat) : OfNat Nat n where
  ofNat := ofNat n

@[simp]
theorem Nat.zero_eq_lit : Nat.zero = 0 := rfl


theorem Nat.succ_neq_zero {n : Nat} : n.succ ≠ 0 :=
  Nat.noConfusion

theorem Nat.succ_inj {a b : Nat} : a.succ = b.succ ↔ a = b :=
  (Nat.succ.injEq a b).to_iff


def Nat.add : (a b : Nat) → Nat
  | zero,    b => b
  | succ a', b => Nat.succ (Nat.add a' b)

instance : Add Nat where
  add := Nat.add

@[simp]
theorem Nat.zero_add (n : Nat) : 0 + n = n := rfl

@[simp]
theorem Nat.succ_add (n m : Nat) : n.succ + m = (n + m).succ := rfl

@[simp]
theorem Nat.add_zero (n : Nat) : n + 0 = n := by
  induction n with
  | zero => rfl
  | succ n' ih => rw [succ_add, ih]

@[simp]
theorem Nat.add_succ (n m : Nat) : n + m.succ = (n + m).succ := by
  induction n with
  | zero => simp
  | succ n' ih => rw [succ_add, ih]; rfl

@[simp]
theorem Nat.add_comm (n m : Nat) : n + m = m + n := by
  induction n with
  | zero => rw [zero_eq_lit, zero_add, add_zero]
  | succ n' ih => rw [add_succ, succ_add, ih]

@[simp]
theorem Nat.add_assoc (a b c : Nat) : (a + b) + c = a + (b + c) := by
  induction c with
  | zero => rw [zero_eq_lit, add_zero, add_zero]
  | succ n' ih => rw [add_succ, add_succ, add_succ, ih]

@[simp]
theorem Nat.add_cancels (a b c : Nat) : a + b = a + c → b = c := by
  induction a with
  | zero => simp
  | succ a' ih =>
      rw [succ_add, succ_add, succ_inj]
      assumption


def Nat.is_pos (n : Nat) := n ≠ 0

theorem Nat.pos_add_is_pos (a b : Nat) : a.is_pos → (a + b).is_pos := by
  intro ha
  induction b with
  | zero => simp; assumption
  | succ b' ih =>
      simp
      exact Nat.succ_neq_zero

theorem Nat.sum_zero_not_pos (a b : Nat) : a + b = 0 → a = 0 ∧ b = 0 := by
  intro h
  constructor
  . by_contra! ha
    have ab_pos := Nat.pos_add_is_pos a b ha
    contradiction
  by_contra! ha
  rw [add_comm] at h
  have ab_pos := Nat.pos_add_is_pos b a ha
  contradiction

theorem Nat.exists_pred (a : Nat) (h : a.is_pos) : ∃ b : Nat, b.succ = a := by
  induction a with
  | zero => contradiction
  | succ a' ih => use a'

end Watson
