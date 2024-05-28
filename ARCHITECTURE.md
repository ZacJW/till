# till's Architecture

till aims to improve executor and future inter-compatibility by defining a kind of intermediate representation that allows executors and futures to be developed in isolation from each other, while maintaining compatibility.

till introduces Subsystems, which are exposed by Executors, which are in turn exposed by Executor Contexts to Futures that need Subsystem support.

till's architecture is capable of supporting `no_std` environments, and even no-alloc though there are special considerations when working in those environments.

## Subsystems

Subsystems manage some portion of the Executor's internal state to allow Futures to perform a particular category of asynchronous operation like waiting, filesystem access, or networking for example. A Subsystem is generally defined by one (or two if both explicit and implicit Executor Contexts are supported) trait(s) that describe the Subsystem itself. There will often be associated types with other traits to bound them for state that Futures must store to make of the Subsystem.

till-modular, the executor framework that till was built to enable, goes further with general traits for all Subsystems as well as concepts like Subsystem Groups.

## Executors

Executors manage scheduling spawned Futures, as well as driving the Subsystems it contains. They generally own the 'main-loop' making themselves blocking to whatever started them, though some may let other application code own the 'main-loop' and be content on being regularly polled.

till leaves the details of how Executors do their scheduling up to the implementor, but till-modular has additional concepts like Marshalls and Task Managers.

## Executor Contexts

Executor Contexts are how a Future is able to communicate with the Executor and the Subsystems it contains. Most popular executors make use of what till calls Implicit Executor Contexts where there is some hidden channel of communication (usually through a global) that Futures make use of, but till also supports (and prefers) Explicit Executor Contexts where the Futures must receive as an argument a reference type which gives it access to the Executor.

The benefit of the implicit form is that it makes your code shorter not having to carry around this value throughout all of your futures. The benefit of the explicit form is that it's a compiler error (rather than a runtime panic) to use a Future on an Executor that doesn't provide the Subsystems it needs.

## Futures

Futures are implemented as generic over the Executor, but bounded by the Subsystem traits it requires. This lets the compiler optimise for the particular executor in use, hopefully making it a zero-cost abstraction. If Explicit Executor Contexts are used it will need to receive that context as an argument. If not it may still be necessary to chose a particular specialisation of the generic. This could involve a turbofish at every call-site or a specialised definition macro. Module-level generic parameters would be another solution but Rust doesn't currently support them.
