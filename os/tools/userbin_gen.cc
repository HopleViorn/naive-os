#include<iostream>
using namespace std;
#define zxc(x) cerr<<(#x)<<'='<<(x)<<'\n';
char* user_c_name[]={
	"init",
	"shell",
	"forktest",
	"argtest",
	"runtest"
};

char* testsuits_name[]={
	"getpid",
	"getppid",
	"read",
	"write",
	"openat",
	"open",
	"close",
	"gettimeofday",
	"sleep",
	"wait",
	"waitpid",
	"times",
	"exit",
	"brk",
	"execve",
	"test_echo",
	"fork",
	"yield",
	"clone",
	"mount",
	"umount",
	"getcwd",
	"getdents",
	"dup",
	"dup2",
	"mkdir_",
	"chdir",
	"fstat",
	"uname",
	"unlink",
	"pipe",
	"mmap",
	"munmap",
	"busybox",
	"busybox_debug",
};

signed main(){
	int len0=sizeof(user_c_name)/8;
	int len1=sizeof(testsuits_name)/8;
	printf("	.align 3\n");
	printf("	.section .data\n");
	printf("	.globl _app_num\n");
	printf("_app_num:\n");
	printf("	.quad %d\n",len0+len1);
	for(int i=0;i<len0;++i){
		char* s=user_c_name[i];
		printf("	.quad %s_start\n",s);
		// printf("	.quad %s_end\n",s);
	}
	for(int i=0;i<len1;++i){
		char* s=testsuits_name[i];
		printf("	.quad %s_start\n",s);
		if(i==len1-1) printf("	.quad %s_end\n",s);
	}
	
	printf("\n");
	printf("	.globl _app_names\n");
	printf("_app_names:\n");
	for(int i=0;i<len0;++i){
		char* s=user_c_name[i];
		printf("	.string \"%s\"\n",s);
	}
	for(int i=0;i<len1;++i){
		char* s=testsuits_name[i];
		printf("	.string \"%s\"\n",s);
	}
	printf("\n");
	for(int i=0;i<len0;++i){
		char* s=user_c_name[i];
		printf("	.section .data\n");
		printf("	.global %s_start\n",s);
		printf("	.global %s_end\n",s);
		printf("	.align 3\n");
		printf("%s_start:\n",s);
		printf("	.incbin \"../user_c/build/%s\"\n",s);
		printf("%s_end:\n",s);
		printf("\n");
	}
	for(int i=0;i<len1;++i){
		char* s=testsuits_name[i];
		printf("	.global %s_start\n",s);
		printf("	.global %s_end\n",s);
		printf("	.align 3\n");
		printf("%s_start:\n",s);
		// printf("	.incbin \"../testsuits/%s\"\n",s);
		printf("	.incbin \"../testbin/%s\"\n",s);
		printf("%s_end:\n",s);
		printf("\n");
	}
	return 0;
}