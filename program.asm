.data
    a DCD 10
.global
_start:
    printr R2
    LDR R0, [a]
loop:
    BL some_procedure
    SUB R0, R0, #1
    ADD R1, R1, #1
    JNZ R0, loop
    EXIT
some_procedure:
    PRINTR R1
    RET