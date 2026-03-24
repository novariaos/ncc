#ifndef NVM_H
#define NVM_H

void __nvm_exit(int code) __attribute__((noreturn));
void __nvm_print(int ch);
int  __nvm_spawn(void);
int  __nvm_open(void);
int  __nvm_read(int fd);
int  __nvm_write(int fd, int byte);
int  __nvm_create(void);
int  __nvm_delete(void);
int  __nvm_cap_request(int pid);
void __nvm_cap_spawn(int pid, int cap);
void __nvm_msg_send(int recipient, int content);
int  __nvm_msg_receive(void);
int  __nvm_inb(int port);
void __nvm_outb(int port, int val);
int  __nvm_tty_fd(void);

#endif
