#include "lib/syscall.h"
#include "lib/unistd.h"
#include "lib/stdio.c"
#include "lib/string.c"
char* argv[16];
char testsuits_name[][49]={
	"mount",
	"fstat",
	"getpid",
	"getppid",
	"write",
	"read",
	"gettimeofday",
	"sleep",
	"wait",
	"waitpid",
	"exit",
	"execve",
	"fork",
	"brk",
	"times",
	"yield",
	"clone",
	"openat",
	"open",
	"close",
	"getcwd",
	"getdents",
	"dup",
	"dup2",
	"mkdir_",
	"chdir",
	"uname",
	"unlink",
	"mmap",
	"pipe",
	"munmap",
	"umount",
};
signed main(){
	// execve("shell",0,0);
	argv[0]="arg0";
	argv[1]="arg1";
	argv[2]="arg2";
	argv[3]="arg3";
	argv[4]=0;
	int test_cc=sizeof(testsuits_name)/49;
	printf("test cases:%d\n",test_cc);
	for(int i=0;i<test_cc;i++){
		if(fork()==0){
			execve(testsuits_name[i],0,0);
		}else {
			wait(0);
		}
	}
	// execve("shell",0,0);
	return 0;
}