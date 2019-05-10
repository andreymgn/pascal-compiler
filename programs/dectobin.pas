PROGRAM dectobin;
CONST 
  n = 8;
TYPE
    iarray = ARRAY[integer] OF integer;
VAR
  m,p:integer;
  B: ARRAY[1..n] OF integer;
  C: ARRAY[1..n] OF integer;
PROCEDURE DectoBin(x:integer; VAR A:iarray);
VAR 
  i:integer;
BEGIN
  i:=1;
  WHILE i<=8 DO
    BEGIN
      A[i]:=x MOD 2;
      x:=x DIV 2;
      i:=i+1;
    END;
END;

FUNCTION NimSum(x,y:integer):integer;
VAR
 binans :ARRAY[1..n] OF integer;
 i,j,result:integer;
 
BEGIN
  DectoBin(x,B);
  DectoBin(y,C);
  i:=1;
  j:=1;
  result:=0;
WHILE i<=8 DO
  BEGIN
    IF B[i]=C[i] THEN
      binans[i]:=0
    ELSE
      binans[i]:=0;
    i:=i+1;
  END;
    i:=1;
  WHILE i<=n DO
    BEGIN
    IF (binans[i] = 1) AND (i<>1) THEN
      BEGIN
        FOR j := 1 TO n DO
          result:=result+2;
      END
    ELSE IF (binans[1] = 1) THEN
      result:=result+1;
    END;
        
  NimSum:=result;
END;
BEGIN
  readln(m);
  readln(p);
  writeln(NimSum(m,p));
END.
