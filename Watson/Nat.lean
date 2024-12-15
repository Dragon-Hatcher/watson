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


end Watson
