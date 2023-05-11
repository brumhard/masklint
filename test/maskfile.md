# Tasks

The following maskfile contains scripts full of linting errors to test masklint against it.

## bash

> runs cargo clippy

```bash
unused="unused"
test=$(echo "test")
mkdir $test
```

## python

```py
import os
name = os.getenv("name", "WORLD");
print("Hello, " + name + "!")
```
