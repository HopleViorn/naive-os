#include "lib/syscall.h"
#include "lib/unistd.h"
#include "lib/stdio.c"
#include "lib/string.c"


signed main(int argc,char* argv[]){
	int fd = open("./text.txt", 0);
	write(fd, "hello new file", 15);
	char buf[256];
	int fd2=open("/text.txt",0);
	int size = read(fd2, buf, 256);
	printf("size=%d\n",size);
	printf("%s\n",buf);
	
	close(fd);
	return 0;
}