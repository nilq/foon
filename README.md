## foon

an interpreter for a wannabe low-level language with weird typing.

### draft

```
i32: a = 10

i32:
  b
  c = 20

i32: sum ([i32]: nums)
  mut i32: acc = 0
  for i in nums
    acc += i

  acc
  
i32: summed = sum {a, b, c}

# stdout => 50
print summed
```
