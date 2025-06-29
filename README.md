# Sparse ECS

A simple sparse-set ECS implementation in Rust, with no unsafe. I use this in small personal projects and cater it to a very specific usecase.

### Features

* Resources for arbitrary thread-safe (rwlock) data access
* World (flexible component storage)
* Tags (static str entity hashset)


### Does not do

* Systems/scheduling
* Complex queries
* Inherently multi-threaded world access
