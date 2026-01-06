# OXIDICE

## how to profile this application

```
cargo flamegraph --bin profile_me --release
```

宽容求值：如果是非法骰子如(-1)d10 或 10d0，则会返回一个空的骰子池，且为 0
其他：对于应当是整数的场合比如 xdy 中 x,y 和修饰器中，min、max 后的数，会通过直接截断的方式转化为整数
