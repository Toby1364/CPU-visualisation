
ldi x ff06
sto x ff04

ldi x 1
mov y

loop:

swp
add

sto x ff00
sto y ff02

lod x ff04
str y

ldi y 2
add
sto x ff04

jz done

lod x ff00
lod y ff02

jmp loop

done:

hlt
