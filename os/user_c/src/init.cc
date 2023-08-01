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
			if(x <= 0) return ;
		}
	}
}
signed main(){
	execute("time-test");
	// execute("busybox sh libctest_testcode.sh");
	execute("busybox sh unixbench_testcode.sh");
	// execute("busybox sh ./test.sh date.lua");
	// execute("busybox sh ./test.sh file_io.lua");
	// execute("busybox sh ./test.sh max_min.lua");
	// execute("busybox sh ./test.sh random.lua");
	// execute("busybox sh ./test.sh remove.lua");
	// execute("busybox sh ./test.sh sin30.lua");
	// execute("busybox sh ./test.sh round_num.lua");
	// execute("busybox sh ./test.sh sort.lua");
	// execute("busybox sh ./test.sh strings.lua");
	// execute("busybox sh busybox_testcode.sh");
	printf("[init proc] done.\n");
	return 0;
}