# How to become a monster engineer while using AI every day

**The research is unambiguous: AI coding assistants accelerate output but degrade learning — unless you deliberately structure your workflow to counteract this.** A landmark 2026 Anthropic randomized trial found that developers using AI scored **17% lower** on conceptual understanding assessments, with no meaningful productivity gain. But the study also identified a specific usage pattern — *Conceptual Inquiry* — where developers scored **86%** by asking AI only "how" and "why" questions while writing all code manually. The difference isn't whether you use AI. It's how. What follows is a concrete system built from learning science research, practitioner evidence, and the workflows of engineers at Google, DeepMind, and other top organizations — tailored for a compiler engineer who wants staff-level judgment, not just shipping velocity.

---

## The core problem: AI removes the friction that builds expertise

Robert Bjork's foundational research on *desirable difficulties* established a counterintuitive truth: **conditions that make learning feel harder produce better long-term retention**, while conditions that feel easy and productive often fail to create lasting knowledge. The four proven desirable difficulties — spacing, interleaving, retrieval practice, and varying conditions — are precisely the kinds of cognitive struggle that AI tools eliminate.

The evidence is piling up fast. The Anthropic study (Shen & Tamkin, January 2026) tracked 52 developers learning an async programming library. AI users completed tasks slightly faster but understood significantly less. The researchers identified six distinct interaction patterns, three of which produced abysmal learning outcomes: *AI Delegation* (outsourcing all coding), *Progressive AI Reliance* (starting with questions but escalating to full delegation), and *Iterative AI Debugging* (using AI to fix errors without understanding them). A separate METR trial found experienced open-source developers were actually **19% slower** with AI on their own codebases — yet believed they were 20% faster.

The mechanism is *cognitive offloading*. When AI generates the code, your brain doesn't perform the generation, retrieval, and error-correction processes that encode knowledge into long-term memory. A 2025 MDPI study of 666 participants found a significant negative correlation between AI tool usage and critical thinking ability, with **younger participants showing the highest dependence and lowest critical thinking scores**. At 22, this is your specific risk.

But the research also reveals a clear path forward. The distinction between productive friction (keep) and unproductive friction (remove with AI) is the key to the entire system.

**Productive friction you must preserve:** debugging your own code, designing solutions before implementing, reading unfamiliar codebases, working through compiler errors, writing tests that force edge-case thinking, struggling with algorithm design, understanding *why* a solution works.

**Unproductive friction AI should handle:** syntax lookup, boilerplate scaffolding, formatting, initial test structure, project configuration, translating between equivalent representations, searching documentation for specific API signatures.

---

## The daily system: a concrete protocol you can start Monday

The system has three layers — a daily workflow for AI-assisted coding sessions, an end-of-day consolidation ritual, and a weekly deepening practice. Each component is backed by specific research and used by engineers at top organizations.

### During coding: the Conceptual Inquiry workflow

This is the single most important habit. It comes directly from the Anthropic study's highest-performing pattern and aligns with Addy Osmani's workflow at Google Chrome. The protocol has five steps:

**Step 1 — Spec before code.** Before touching any code, brainstorm a detailed specification with AI. Osmani calls this "waterfall in 15 minutes." Ask the AI to iteratively question you about requirements, edge cases, and constraints. Compile this into a `spec.md`. For LLVM work, this means writing out what your pass should transform, what invariants it must preserve, what IR patterns it targets, and what the expected output looks like — before generating a single line.

**Step 2 — Attempt first, AI second.** For any non-trivial logic, spend 15–30 minutes writing your own implementation before consulting AI. This triggers the *generation effect* (Slamecka & Graf, 1978): information you actively produce is retained far better than information you passively read. After your attempt, ask AI for its version. Compare the two. The delta between your solution and the AI's is your learning edge — the exact boundary of your current understanding.

**Step 3 — Ask "why," not "write."** When you do use AI for help, ask conceptual questions: "Why does LLVM use a use-def chain here instead of a def-use chain?" or "What are the tradeoffs between a greedy register allocator and a graph-coloring one?" Never ask "write me a register allocator." The Anthropic data is stark: developers who asked conceptual questions and coded manually scored more than double those who delegated code generation.

**Step 4 — Type, don't paste.** The Anthropic study found that participants who manually typed AI-suggested code retained more understanding than those who copy-pasted. The physical act of typing engages deeper processing. When you do use AI-generated code, retype it. Modify it as you go. This sounds trivial. The retention difference is not.

**Step 5 — Treat every AI output as a junior's PR.** Osmani's explicit framing: "I am the senior dev; the LLM is there to accelerate me, not replace my judgment." Read every line. Run it. Test it. The Nilenso consultancy (a decade-old shop) puts it bluntly: "AI is a multiplier. If you are a small coefficient, you won't see much gain. If you are a negative coefficient, expect negative gains."

### End of day: the 15-minute consolidation ritual

Mark Erikson, maintainer of Redux, has kept a daily engineering journal since 2013. His practice takes **10–15 minutes** at end of day and has compounded into one of the most valuable knowledge assets in his career. Adapt his system:

Open a markdown file (one per day, organized by month). Write three things:

**What I built and why.** Not just "implemented X pass." Write: "Implemented a dead code elimination pass that operates on LLVM IR by identifying instructions with no uses and no side effects. The key insight was handling PHI nodes at block boundaries — initially I missed that a PHI node can appear unused in its own block but be the sole input to a critical edge."

**What I learned that was new.** This is your TIL (Today I Learned). Capture the specific concept, not just the fact. "Learned that LLVM's `MemorySSA` provides a virtual SSA form for memory operations, which lets you query memory dependencies without computing full alias analysis. This is faster for passes that only need local memory ordering."

**What I'd do differently.** The retrospective trigger. "Spent 45 minutes debugging a segfault that turned out to be iterator invalidation during instruction deletion. Should have used `make_early_inc_range` from the start — adding this to my pre-coding checklist."

This journal doubles as your context file for AI sessions (save it as `CLAUDE.md` or equivalent) and as ammunition for performance reviews. The real value is that **writing forces retrieval and elaboration** — two of the highest-utility learning techniques identified in Dunlosky et al.'s landmark 2013 meta-analysis.

### Weekly: the deepening practice

Every Friday, spend 30 minutes on three activities:

**Git log review.** Run `git log --author="you" --since="1 week ago" --oneline`. Open your merged PRs. Scan for patterns — repeated logic, inconsistent approaches, areas where you relied heavily on AI. A DEV Community engineer who adopted this practice reported catching architectural inconsistencies they'd been blind to during daily work.

**One re-implementation.** Pick one piece of code you wrote with significant AI assistance during the week. Rewrite it from scratch without AI. This is pure retrieval practice — the single most effective learning technique in the cognitive science literature. You'll discover exactly which parts you understood and which you were cargo-culting. For LLVM work, re-implement a pass you wrote, or manually walk through an optimization you applied.

**One Feynman explanation.** Take the most complex concept you encountered that week and write a 200-word explanation as if teaching it to someone who knows C++ but not compilers. If you can't explain why LLVM represents SSA form the way it does, or why your pass needs to handle critical edges, you don't fully understand it yet. Post these publicly (a GitHub TIL repo, a blog, or even a tweet thread). Shawn Wang's "Learn in Public" philosophy is backed by the research: **teaching is the highest-retention learning activity**.

---

## The Janki Method: spaced repetition without the overhead

Jack Kinsella's Janki Method is the most battle-tested system for applying spaced repetition to programming. Derek Sivers (founder of CDBaby) calls it "the most helpful learning technique I've found in 14 years of programming." The system takes **20 minutes per morning** once established. Here's the version adapted for compiler engineering:

**Rule 1: Every new concept becomes a card.** When you learn that `llvm::SmallVector` defaults to stack allocation below N elements and switches to heap above, make a card. Front: "What happens when SmallVector exceeds its inline capacity?" Back: "Heap allocation, amortized O(1) push_back, but may invalidate iterators."

**Rule 2: Every bug becomes a card.** After debugging, create a card for the *class* of error, not the specific instance. Front: "What's the most common cause of crashes during LLVM pass iteration?" Back: "Iterator invalidation — deleting or inserting instructions while iterating. Use make_early_inc_range or collect-then-modify pattern."

**Rule 3: Keep cards atomic and concrete.** Not "explain SSA form" but "In SSA form, what happens when a variable is assigned in two different branches?" Use code snippets on cards. Use cloze deletions for IR patterns.

**Rule 4: Review every morning.** 20 minutes with coffee. The spacing algorithm handles the scheduling. Cards you know well appear rarely; cards you struggle with appear daily. After 3–6 months, you'll have a searchable library of hundreds of compiler engineering concepts that you can actually recall under pressure.

**Rule 5: Prune ruthlessly.** Delete cards that don't connect to your actual work. Suspend cards that frustrate you — multi-step processes don't Ankify well. The goal is a lean deck of high-value knowledge atoms.

The research basis is strong. Karpicke & Roediger (2008) demonstrated that retrieval practice significantly outperforms re-studying, and spaced retrieval dramatically outperforms massed retrieval. For a compiler engineer, this means the concepts you review in Anki will be available instantly when you're debugging a miscompilation at 3 PM — not something you vaguely remember reading once.

---

## Building staff-level judgment: what the research actually says

Engineering judgment — the intuition that lets a staff engineer say "this design won't scale" before anyone writes a line of code — develops through a specific set of cognitive mechanisms: **pattern recognition, mental model construction, anomaly detection, and metacognition**. Academic research from the ASEE is blunt: "There is consensus that engineering judgment is the product of experience in an authentic engineering environment."

AI can support or undermine this development depending entirely on how you use it. The risk: if AI generates your designs and you just review them, you never build the pattern library that constitutes judgment. The opportunity: if you use AI to explore more design alternatives than you could manually, you accelerate pattern acquisition.

**Five practices that build compiler engineering judgment specifically:**

**Read LLVM source with intent.** Not passively browsing — pick a specific pass (e.g., `InstCombine`, `GVN`, `LoopVectorize`) and trace its logic from entry to exit. Ask yourself before reading each function: "What do I think this does?" Then verify. This is interleaved retrieval practice applied to code reading. Alex Denisov's LLVM learning path recommends starting with the `llvm-tutor` and `clang-tutor` repos, which provide structured exercises that force deep engagement with LLVM internals.

**Write Architecture Decision Records.** For every non-trivial design choice, document: what you decided, what alternatives you considered, what constraints drove the decision, and what you expect to happen. Review these quarterly. The gap between your predictions and reality is your judgment calibration signal. Amazon's principal engineers maintain extensive decision logs; AWS teams with mature ADR practices report spending 20–30% less time on coordination.

**Study failure modes obsessively.** Read postmortems from Google's SRE book, LLVM's bug tracker, and GCC's regression archives. Understanding how compiler optimizations *break* teaches you more about correctness constraints than understanding how they succeed. Every miscompilation bug is a lesson in the gap between specification and implementation.

**Implement before abstracting.** When learning a new compiler technique — say, polyhedral optimization or superword-level parallelism — implement a naive version from scratch before reading the production implementation. The struggle of building it wrong, then seeing how the production version solves the problems you encountered, creates far deeper understanding than reading the elegant solution first. Scott Hanselman's principle: "If your system is a magical black box to you, demystify it. Crack it open."

**Practice making trade-off decisions explicitly.** Staff engineers don't just pick the "best" solution — they pick the *right* solution for the context. When designing an LLVM pass, articulate: "I'm choosing a worklist algorithm over a recursive traversal because the CFG may have exponential paths, accepting higher memory usage for bounded runtime." Revisit these decisions after 6 months. The calibration between predicted and actual consequences is how judgment forms.

---

## The compiler engineer's specific advantage

You're in an unusually good position. Compiler engineering is one of the domains where AI tools are least likely to replace deep understanding, because **correctness constraints are absolute** — a miscompilation isn't a UI bug, it's a silent data corruption that may not surface for months. This means the market will increasingly reward the judgment you're building, even as AI commoditizes simpler coding tasks.

Your weekly learning should include elements specific to this domain. Cornell's CS 6120 (Adrian Sampson's Advanced Compilers course) assigns academic papers and has students implement the concepts — the materials are freely available online. Denis Bakhvalov's "Performance Analysis and Tuning on Modern CPUs" bridges the gap between compiler output and hardware behavior. The LLVM Weekly newsletter (Alex Bradbury) keeps you current on developments in the ecosystem. And the LLVM YouTube channel archives every developer meeting talk.

The critical habit: **understand one abstraction level below your daily work**. If you work on middle-end optimization passes, understand the backend's instruction selection and register allocation well enough to predict how your IR transformations affect generated machine code. If you work on codegen, understand cache hierarchies and branch prediction well enough to evaluate whether your scheduling decisions actually improve performance. This vertical depth is what separates compiler engineers who optimize benchmarks from those who optimize real programs.

---

## Conclusion: the system in one page

The research converges on a single insight: **learning happens through effortful retrieval, not effortless consumption**. AI makes consumption effortless. Your system must reintroduce effortful retrieval at every stage.

**During coding:** Spec first. Attempt before asking. Ask "why" not "write." Type, don't paste. Review like a senior reviewing a junior's PR.

**End of day (15 min):** Journal what you built, what you learned, and what you'd change. Save as context for tomorrow's AI sessions.

**Every morning (20 min):** Anki review of compiler concepts, bug classes, and LLVM patterns.

**Every Friday (30 min):** Git log review, one AI-free re-implementation, one Feynman explanation written publicly.

**Ongoing:** Read LLVM source with intent. Write ADRs. Study failures. Implement before abstracting. Understand one level deeper.

The engineers who will thrive aren't those who avoid AI or those who delegate everything to it. They're the ones who use AI to move faster while deliberately preserving the cognitive struggle that builds genuine expertise. At 22, working on LLVM, with this system running daily — you have a decade-long compounding advantage that most engineers will never build. The friction is the feature.
