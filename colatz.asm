ldi x 0A         ; Start the sequence with number 10 (or any other initial value)
sto x FF00       ; Store the initial value in memory address FF00

ldi x 00         ; Initialize step counter to 0
sto x FF02       ; Store the counter at memory address FF02

loop:
lod x FF00       ; Load the current value of the sequence from memory

ldi y 01         ; Load immediate value 1 into Y
sub              ; Subtract Y from X (X - 1)
jz done          ; If X == 0 (i.e. the value is 1), jump to halt

; Increment the step counter
lod x FF02       ; Load the current counter value from memory
ldi y 01         ; Load immediate value 1 into Y
add              ; Increment the counter (X = X + 1)
sto x FF02       ; Store the updated counter back in memory

; Reload the sequence value for further operations
lod x FF00       ; Load the current value of the sequence from memory

ldi y 02         ; Load immediate value 2 into Y
mod              ; X = X % 2
jz even          ; If X % 2 == 0, jump to the even block

; Odd case: Calculate 3X + 1
lod x FF00       ; Reload X from memory (current sequence number)
ldi y 03         ; Load immediate value 3 into Y
mul              ; X = X * 3
ldi y 01         ; Load immediate value 1 into Y
add              ; X = X + 1
sto x FF00       ; Store the new value of X in memory
jmp loop         ; Jump back to the start of the loop

even:
lod x FF00       ; Reload X from memory
ldi y 02         ; Load immediate value 2 into Y
div              ; X = X / 2
sto x FF00       ; Store the new value of X in memory
jmp loop         ; Jump back to the start of the loop

done:
hlt              ; Halt the program when X reaches 1
