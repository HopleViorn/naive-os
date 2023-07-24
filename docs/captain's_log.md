## On Supporting MUSL LIBC
### 1:6/28
Glibc can't load value from specific memory location.

#### Analysis:
Infact the value can be loaded correctly as it is mapped correctly, but the value loaded from the location is always $0$. After several days's of debugging, we found that it is the problem with data copying at pagetable mapping stage. One segment in busybox's elf file is not page-size aigned, but the copying procedure implemented by rcore only copies the data in a page-aligned way. After support offset copying the problem solved.

### 2:6/29
The Initial stack is not correctly arranged in the way glibc requires. 
#### Analysis:
Glibc requires passing arguments by order as argc, argv, env, auxvec. We do not support env and auxvec at the beginning. Adding an empty env and auxvec pointer didn't solve the problem really much. Busybox keeps accessing the address 0x1. In fact, it is the problem with auxvec AT_RANDOM, which is a must-supported for canarry,for buffer-overflow-attack-proof. The parameter should be a pointer to a 16-bit random value and must be included in the auxvec. After adding this auxvec, glibc can load correct value.

#### 3:6/28
Page fault occurs after a brk syscall.
#### Analysis
brk() is not implemented correctly. After fixing brk the page fault did not occur.

#### 4:7/24
Musl Alloc/Free not performing well.
#### Analysis
Encountered the same problem FTL meets. FTL's debugging log really helps. It is the matter with auxvec AT_PAGESIZE(you again).

## On Supporing Multi-Core
### 1:6/29
Forget to set satp on each core.