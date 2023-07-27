#include "lib/unistd.h"
#include "lib/stdio.c"

// char* argv[19]={"busybox","sh","max.sh"};
char* argv[19]={"busybox","sh","busybox_testcode.sh"};
// char* argv[19]={"busybox","sh","lua_testcode.sh"};

signed main(){
	printf("[init proc] hello!\n");
	if(fork()==0){
		// execve("write", argv, 0);
		execve("busybox", argv, 0);
		// execve("busybox", argv, 0);
		// execve("/core/shell", argv, 0);
		
		// execve("clone", 0, 0);
			// execve("shell", 0, 0);
		// execve("runtest", 0, 0);
		// execve("shell", 0, 0);
		// execve("runtest", 0, 0);
		// execve("mount", 0, 0);
	}else{
		for(;;){
			int status;
			int x=waitpid(-1,&status,0);
			if(x<0) break;
			printf("[init proc] get pid=%d exited.\n",x);
		}
		printf("[init proc] done.");
	}
	return 0;
}