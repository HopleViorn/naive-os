#include "lib/unistd.h"
#include "lib/stdio.c"
char* argv[99];

void execute(char* buf){
	char *cc=buf;
	int argc=0;
	argv[0]=buf;
	while(*cc){
		if(*cc==' '){
			*cc=0;
			argv[++argc]=cc+1;
		}
		cc++;
	}
	argv[++argc]=0;
	if(fork()==0){
		execve(buf, argv, 0);
	}else{
		for(;;){
			int status;
			int x=waitpid(-1,&status,0);
			if(x < 0) return ;
		}
	}
}
signed main(){
	execute("time-test");
	execute("busybox sh busybox_testcode.sh");
	// execute("busybox sh lua_testcode.sh");
	printf("[init proc] done.\n");
	return 0;
}