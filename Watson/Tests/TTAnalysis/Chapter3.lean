import Watson.Set
import Mathlib

namespace Watson

/-! # Chapter 3: Set theory -/

universe u
variable {α β : Type u}
variable (a b c : Set α)

/-! ## Section 3.1: Fundamentals -/

-- Many of the statements in this chapter don't map cleanly on to Lean where
-- the foundations of how sets are defined are not the same. I've commented out
-- any statements that don't make sense in Lean and written a brief explanation.

-- theorem axiom_3_1 : sets are objects :=
--   Anything you can mention in Lean is already an object in the sense that
--   Tao means so there isn't a meaningful way of stating this.

-- Note that for Tao this is a definition but since Lean defines equality
-- separately, for us this is a theorem based on functional/propositional
-- extension. We also only prove it one way in the library since Lean can
-- handle the forward direction with a rewrite (what Tao calls the axiom of
-- substitution).
theorem def_3_1_4 (s₁ s₂ : Set α) : s₁ = s₂ ↔ ∀ x, x ∈ s₁ ↔ x ∈ s₂ := by
  constructor
  · intro h; rw [h]; simp
  exact Set.ext

-- theorem ex_3_1_1 : set equality is reflexive, symmetric, and transitive :=
--   Again equality is already defined for use and Lean already has proofs of
--   these facts built in. We could prove it explicitly for the above definition
--   but that would be basically a waste of time.

theorem axiom_3_2 : ∃ (empty : Set α), ∀ x, x ∉ empty := by
  use ∅
  exact Set.not_in_empty

-- Note that whereas Tao defines non empty sets as being those not equal to the
-- empty set and then proves those sets contain an element, we define it the
-- other way around and so our lemma 3.1.6 is the opposite of his.
theorem lemma_3_1_6 (s : Set α) (h : s.non_empty) : s ≠ ∅ :=
  (Set.non_empty_iff_ne_empty s).mp h

theorem axiom_3_3 (a : α) : ∃ (s : Set α), ∀ x, x ∈ s ↔ x = a := by
  use {a}; intro x; rfl

theorem axiom_3_4
  : ∃ (ab : Set α), ∀ x, x ∈ ab ↔ (x ∈ a ∨ x ∈ b) := by
  use a ∪ b; intro x; rfl

theorem lemma_3_1_13_a (a b : α) : ({a, b} : Set α) = {a} ∪ {b} := by rfl
theorem lemma_3_1_13_b : a ∪ b = b ∪ a := Set.union_comm a b
theorem lemma_3_1_13_c : (a ∪ b) ∪ c = a ∪ (b ∪ c) := Set.union_assoc _ _ _
theorem lemma_3_1_13_d : a ∪ a = a := Set.union_self a
theorem lemma_3_1_13_e : a ∪ ∅ = a := Set.union_empty a
theorem lemma_3_1_13_f : ∅ ∪ a  = a := Set.empty_union a

theorem example_3_1_17_a : a ⊆ a := Set.sub_rfl a
theorem example_3_1_17_b : ∅ ⊆ a := Set.empty_sub a

theorem prop_3_1_18_a (hab : a ⊆ b) (hbc : b ⊆ c) : a ⊆ c :=
  Set.sub_trans hab hbc
theorem prop_3_1_18_b (hab : a ⊆ b) (hba : b ⊆ a) : a = b :=
  Set.sub_anti_rfl hab hba
theorem prop_3_1_18_c (hab : a ⊂ b) (hbc : b ⊂ c) : a ⊂ c :=
  Set.ssub_trans hab hbc

theorem axiom_3_5 (motive : α → Prop) (s₁ : Set α)
  : ∃ (s₂ : Set α), ∀ x, x ∈ s₂ ↔ (x ∈ s₁ ∧ motive x) := by
  use { x ∈ s₁ | motive x }
  intro x
  rfl

theorem prop_3_1_28_a_i  : a ∪ ∅ = a := Set.union_empty a
theorem prop_3_1_28_a_ii : ∅ ∪ a = a := Set.empty_union a
theorem prop_3_1_28_b_i  : a ∪ Set.univ = Set.univ := Set.union_univ a
theorem prop_3_1_28_b_ii : Set.univ ∪ a = Set.univ := Set.univ_union a
theorem prop_3_1_28_c_i  : a ∩ a = a := Set.inter_self a
theorem prop_3_1_28_c_ii : a ∪ a = a := Set.union_self a
theorem prop_3_1_28_d_i  : a ∪ b = b ∪ a := Set.union_comm a b
theorem prop_3_1_28_d_ii : a ∩ b = b ∩ a := Set.inter_comm a b
theorem prop_3_1_28_e_i  : (a ∪ b) ∪ c = a ∪ (b ∪ c) := Set.union_assoc a b c
theorem prop_3_1_28_e_ii : (a ∩ b) ∩ c = a ∩ (b ∩ c) := Set.inter_assoc a b c
theorem prop_3_1_28_f_i  : a ∩ (b ∪ c) = (a ∩ b) ∪ (a ∩ c) :=
  Set.inter_union_distrib_left a b c
theorem prop_3_1_28_f_ii : a ∪ (b ∩ c) = (a ∪ b) ∩ (a ∪ c) :=
  Set.union_inter_distrib_left a b c
theorem prop_3_1_28_g_i  : a ∪ (Set.univ \ a) = Set.univ := by
  rw [← Set.compl_eq_univ_diff]
  exact Set.self_union_compl a
theorem prop_3_1_28_g_ii : a ∩ (Set.univ \ a) = ∅ := by
  rw [← Set.compl_eq_univ_diff]
  exact Set.self_inter_compl a
theorem prop_3_1_28_h_i
  : Set.univ \ (a ∪ b) = (Set.univ \ a) ∩ (Set.univ \ b) := by
  simp only [← Set.compl_eq_univ_diff]
  exact Set.compl_union a b
theorem prop_3_1_28_h_ii
  : Set.univ \ (a ∩ b) = (Set.univ \ a) ∪ (Set.univ \ b) := by
  simp only [← Set.compl_eq_univ_diff]
  exact Set.compl_inter a b

theorem axiom_3_6 (motive : α → β → Prop) (s₁ : Set α)
  : ∃ (s₂ : Set β), ∀ x, x ∈ s₂ ↔ ∃ y ∈ s₁, motive y x := by
  use { x | ∃ y ∈ s₁, motive y x }
  intro x
  rfl

-- theorem axiom_3_7 : there exists the set of all natural numbers :=
--   This set is just (Set Nat).univ but stating all the properties of this set
--   as Tao does would not be simple.

theorem ex_3_1_2_a : (∅ : Set (Set Prop)) ≠ {∅} := by
  intro h
  have h₁ : ∅ ∉ ∅ := Set.not_in_empty (∅ : Set Prop)
  have h₂ : (∅ : Set Prop) ∈ ({(∅ : Set Prop)} : Set (Set Prop)) := rfl
  rw [← h] at h₂
  contradiction
-- theorem ex_3_1_2_b : ∅     ≠ {{∅}}   :=
-- theorem ex_3_1_2_c : ∅     ≠ {∅,{∅}} :=
-- theorem ex_3_1_2_d : {∅}   ≠ {{∅}}   :=
-- theorem ex_3_1_2_e : {∅}   ≠ {∅,{∅}} :=
-- theorem ex_3_1_2_f : {{∅}} ≠ {∅,{∅}} :=
--   These are just a mess and all the same. For the first two nothing is in the
--   the empty set but the right sets have something in them. For the last three
--   first prove that one set contains an element that the other doesn't by the
--   same technique in the previous parts and then the contrapositive of axiom
--   3.3 tells us that this means the sets are unequal.

theorem ex_3_1_5_a : a ⊆ b ↔ a ∪ b = b := Iff.symm Set.union_eq_right
theorem ex_3_1_5_b : a ⊆ b ↔ a ∩ b = a := Iff.symm Set.inter_eq_left

theorem ex_3_1_17_a_i  : a ∩ b ⊆ a := Set.inter_sub_left
theorem ex_3_1_17_a_ii : a ∩ b ⊆ b := Set.inter_sub_right
theorem ex_3_1_17_b    : c ⊆ a ∧ c ⊆ b ↔ c ⊆ a ∩ b := Iff.symm Set.sub_inter_iff
theorem ex_3_1_17_c_i  : a ⊆ a ∪ b := Set.left_sub_union
theorem ex_3_1_17_c_ii : b ⊆ a ∪ b := Set.right_sub_union
theorem ex_3_1_17_d    : a ⊆ c ∧ b ⊆ c ↔ a ∪ b ⊆ c := Iff.symm Set.union_sub_iff

theorem ex_3_1_8_a : a ∩ (a ∪ b) = a := Set.inter_union_self
theorem ex_3_1_8_b : a ∪ (a ∩ b) = a := Set.union_inter_self

theorem ex_3_1_9_a (h₁ : a ∪ b = Set.univ) (h₂ : a ∩ b = ∅) : a = Set.univ \ b := by
  rw [Set.diff_union_compl]
  exact Set.union_univ_inter_empty h₁ h₂
theorem ex_3_1_9_b (h₁ : a ∪ b = Set.univ) (h₂ : a ∩ b = ∅) : b = Set.univ \ a := by
  rw [Set.diff_union_compl];
  apply (Set.compl_eq_iff_eq_compl _ _).mp
  exact Eq.symm (Set.union_univ_inter_empty h₁ h₂)

theorem ex_3_1_10_a : Set.disjoint (a \ b) (a ∩ b) := Set.diff_disjoint_inter
theorem ex_3_1_10_b : Set.disjoint (a \ b) (b \ a) := Set.diff_disjoint_diff
theorem ex_3_1_10_c : Set.disjoint (b \ a) (a ∩ b) := by
  rw [Set.inter_comm]
  exact Set.diff_disjoint_inter
theorem ex_3_1_10_d : (a \ b) ∪ (a ∩ b) ∪ (b \ a) = a ∪ b := by
  apply Set.ext
  intro x
  constructor
  · intro h
    rcases h with ⟨⟨ha, hnb⟩ | ⟨ha, hb⟩⟩ | ⟨hb, hna⟩
    · left; assumption
    · left; assumption
    · right; assumption
  intro h
  cases h with
  | inl ha => cases (em (x ∈ b)) with
    | inl hb  => left; right; exact ⟨ha, hb⟩
    | inr hnb => left; left;  exact ⟨ha, hnb⟩
  | inr hb => cases (em (x ∈ a)) with
    | inl ha  => left; right; exact ⟨ha, hb⟩
    | inr hna => right; exact ⟨hb, hna⟩

theorem ex_3_1_11 :
  (∀ (motive₁ : α → α → Prop) (s₁ : Set α),
   ∃ (s₂ : Set α), ∀ x, x ∈ s₂ ↔ ∃ y ∈ s₁, motive₁ y x) →
  (∀ (motive₂ : α → Prop) (s₃ : Set α),
   ∃ (s₄ : Set α), ∀ x, x ∈ s₄ ↔ (x ∈ s₃ ∧ motive₂ x)) := by
  intro h motive₂ s₃
  have ⟨s₂, hs₂⟩ := h (fun y ↦ fun x ↦ y = x ∧ motive₂ x) s₃
  use s₂
  intro x
  have h₃ := hs₂ x
  constructor
  · intro hxs₂
    have ⟨y, ⟨hys₃, hxy, hmx⟩⟩ := h₃.mp hxs₂
    rw [hxy] at hys₃
    exact ⟨hys₃, hmx⟩
  intro ⟨hx₃, hmx⟩
  exact h₃.mpr ⟨x, ⟨hx₃, rfl, hmx⟩⟩


/-! ## Section 3.1: Russell's paradox -/

-- Interestingly, and instructively, it is not possible to state the axiom of
-- universal specification or thus to encounter Russell's paradox in Lean due
-- to the existence of Lean's hierarchy of types. This is presumably what Tao
-- means when he mentions a hierarchy of objects. Therefore we cannot state the
-- results in this chapter.

end Watson
