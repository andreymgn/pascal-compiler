PROGRAM BubbleSort;
VAR
    i, j, x: integer;
    a: ARRAY[0..5] OF integer;
BEGIN
    a[0] := 3;
    a[1] := 25;
    a[2] := 2;
    a[3] := 69;
    a[4] := 1;

    FOR i := 0 TO 4 DO
    BEGIN
        FOR j := 0 TO 4 DO
            IF a[j] > a[j+1] THEN
            BEGIN
                x := a[j];
                a[j] := a[j+1];
                a[j+1] := x;
            END;
    END;

    FOR i := 0 TO 5 DO
        writeln(a[i]);
END.
