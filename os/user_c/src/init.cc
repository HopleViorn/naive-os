#include "lib/unistd.h"
#include "lib/stdio.c"

char* argv[19]={"busybox","echo","233","ppppp","nuclear"};

signed main(){
	printf("[init proc] hello!\n");
	if(fork()==0){
		// execve("clone", 0, 0);
		execve("busybox_debug", argv, 0);
		// execve("shell", 0, 0);
		// execve("runtest", 0, 0);
		// execve("shell", 0, 0);
		// execve("runtest", 0, 0);
		// execve("mount", 0, 0);
	}else{
		for(;;){
			int status;
			int x=waitpid(-1,&status,0);
			printf("[init proc] get pid=%d exited.\n",x);
		}
		for(;;);
	}
	return 0;
}