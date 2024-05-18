---
title: How to execute various Languages in Presenterm
author: AnirudhG07, mfontanini
---

# Code execution in Presenterm
Presenterm provides a code execution dynamically within the presentation using bash execution.

# General Bash Code Execution
Run commands from the presentation and display their output dynamically using `bash +exec`.

```bash +exec
for i in $(seq 1 5)
do
    echo "Hello $i"
    sleep 0.5
done
```

<!-- end_slide -->

## How to code for Other Languages
We can write codes for various language within the bash script.

_Note_: 
1) The whole code will be printed in the presentation and hence long codes might be unfavourable for ideal presentations.
2) Addition of `#!/bin/bash` is necessary. 
3) Use `ctrl e` to run the code.
<!-- pause -->
# Python 
We can run python using bash script and execute python codes using the below format.
Within the triple quotes you can write your whole python code.
```bash +exec
#!/bin/bash
python -c """
import time

for i in range(5):
    print(f'Hello {i}', flush=True) 
    # flush=True might be necessary for delay
    time.sleep(0.5) 
""" 
```  

<!-- end_slide -->

# Javascript
You can run Javascript using the following format.

```bash +exec
#!/bin/bash
node -e "
for(let i = 1; i <= 5; i++) {
    console.log('Hello ' + i);
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
}
"
```
<!-- end_slide -->
## Other languages
For some languages, the idea is to create a temporary file which stores the code, runs the code and then executes it.
_Note_: One has two options of going about this-
1) Have a pre made code, as long as you want and simply executing it using `bash +exec`. This will prevent display of long codes which you can show otherwise.
2) Make a small temporary function which you can display and execute on the slide itself
The code should be between the `EOF` delimiter.
<!-- pause -->
# Java 
You can run Java code using the following format.

```bash +exec
#!/bin/bash

# Write and run Java code
cat > java_code.java << EOF
public class java_code {
    public static void main(String[] args) {
        for(int i = 1; i <= 5; i++) {
            System.out.println("Hello " + i);
            try {
                Thread.sleep(500); // Sleep for 500 ms
            } catch (InterruptedException e) {
                e.printStackTrace();
            }
        }
    }
}
EOF
javac java_code.java
java java_code
rm java_code.java java_code.class
```

<!-- end_slide -->
## Language Categories
The following is a list where you need to/need not make a temporary file-
<!-- pause -->
# Need Not
1) Python
2) Ruby
3) Perl
4) PHP
5) JavaScript (Node.js)
6) Shell script (bash, sh, zsh, etc.)
7) R
8) Lua
9) Groovy
10) Scala
11) Swift, etc.
<!-- pause -->
# Need To
1) Java
2) C
3) C++
4) C#
5) Go
6) Rust
7) TypeScript (needs to be transpiled to JavaScript first)
8) Kotlin, etc.

<!-- end_slide -->
## More Examples
# Rust
You can run Rust using the following format.
```bash +exec
#!/bin/bash

cat > rust_code.rs << EOF
use std::{thread, time};

fn main() {
    for i in 1..6 {
        println!("Hello {}", i);
        thread::sleep(time::Duration::from_millis(500));
    }
}
EOF
rustc rust_code.rs
./rust_code
rm rust_code.rs rust_code
```
<!-- end_slide -->

# Go

You can run Go using the following format.
```bash +exec
#!/bin/bash

cat > go_code.go << EOF
package main

import (
    "fmt"
    "time"
)

func main() {
    for i := 1; i <= 5; i++ {
        fmt.Printf("Hello %d\n", i)
        time.Sleep(500 * time.Millisecond)
    }
}
EOF
go run go_code.go
rm go_code.go
```
<!-- end_slide -->

# C

You can run C using the following format.
```bash +exec
#!/bin/bash

# Write and run C code
cat > c_code.c << EOF
#include <stdio.h>
#include <unistd.h>

int main() {
    for(int i = 1; i <= 5; i++) {
        printf("Hello %d\n", i);
        fflush(stdout); // Flush the output buffer
        usleep(500000); // Sleep for 500 milliseconds
    }
    return 0;
}
EOF
gcc c_code.c -o c_code
./c_code
rm c_code.c c_code
```

<!-- end_slide -->

# C++

You can run C++ using the following format.
```bash +exec
#!/bin/bash

# Write and run C++ code
cat > cpp_code.cpp << EOF
#include <iostream>
#include <chrono>
#include <thread>
int main() {
    for(int i = 1; i <= 5; i++) {
        std::cout << "Hello " << i << std::endl;
        std::this_thread::sleep_for(std::chrono::milliseconds(500));
        // Sleep for 500 milliseconds
    }
    return 0;
}
EOF
g++ cpp_code.cpp -o cpp_code
./cpp_code
rm cpp_code.cpp cpp_code
```
<!-- end_slide -->
## Other languages
Similarly You can create for other languages as you wish.

### THANK YOU!