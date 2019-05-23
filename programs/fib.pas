PROGRAM fib;
VAR i, a, b, tmp: integer;
BEGIN
    a := 0;
    b := 1;
    FOR i := 0 TO 20 DO
    BEGIN
        writeln(a);
        tmp := a;
        a := b;
        b := tmp + b;
    END;
END.
