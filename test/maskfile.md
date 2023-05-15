# Tasks

The following maskfile contains scripts full of linting errors to test masklint against it.

## bash

```bash
mkdir $unset
```

## python

```py
import os
name = os.getenv("name", "WORLD");
print("Hello, " + name + "!")
```

## ruby

```ruby
name = ENV["name"] || "WORLD"
puts "Hello, #{name}!"
```
