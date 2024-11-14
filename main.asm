ldi x ff04
sto x ff02

ldi x 2
mov y

loop:

mul

sto x ff00

lod x ff02
add
sto x ff02

lod y ff00
str y

lod x ff00
ldi y 3

jmp loop
