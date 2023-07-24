#include "lib/syscall.h"
#include "lib/unistd.h"
#include "lib/stdio.c"
#include "lib/string.c"

// int plus(int a,int b) {
// 	return a+b;
// }
// char ss[]="23333";

char* argv[16];
char buf[256];

signed main() {
	// printf("%s",ss);
	argv[0] = "busybox";
	argv[1] = "arg1";
	argv[2] = "arg2";
	argv[3] = "arg3";
	argv[4] = 0;
	printf("----------shell started!---------\n");
	int cc = 0;
	while(1) {
		printf("shell %d,line %d>", getpid() ,++cc);
		char c = getchar();
		char *t = buf;
		while(c != 13) {
			if(c == 0x08 || c == 0x7f) {
				if(t != buf) {
					printf("\x08 \x08");
					*t-- = 0;
				}
			} else {
				putchar(c);
				*t++ = c;
			}
			c = getchar();
		}
		*t = 0;
		printf("\n");
		if(strlen(buf) == 0) continue;
		if(!strcmp(buf, "sexit")) break;
		if(!strcmp(buf, "syield")) {
			sched_yield();
			continue;
		}
		int pid = fork();
		if(pid == 0) {
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
			execve(buf, argv, 0);
		} else {
			int status;
			wait(&status);
			printf("[shell] exec exited with %d.\n", status >> 8);
		}
	}
	return 0;
}