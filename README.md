# Sparse ECS

A simple sparse-set ECS implementation in Rust, with no unsafe. I use this in small personal projects and cater it to a very specific usecase.

### Features

* Resources for arbitrary thread-safe (rwlock) data access
* World (flexible component storage)
* Tags (static str entity hashset)
* Entity ID re-use


### Does not do

* Systems/scheduling - systems should just be functions, to me. So write some functions.
* Complex queries - TODO! Some macros for mixed mutability access would be convenient.
* Inherently multi-threaded world access - TODO!
