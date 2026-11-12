      * Stage 2, S2.11: fixed-form MOVE and MOVEL lowered to Rust.
      * Expected: what IBM i 7.5 gave (golden capture tools/golden/native8.txt
      * n8_move, cited in native_units.rs).
     H DFTACTGRP(*NO)
     DC5               S              5A
     DX3               S              3A   INZ('XYZ')
     DP3               S              3P 0 INZ(123)
     DN3               S              3P 0 INZ(-42)
     DQ3               S              3P 0
     DP52              S              5P 2 INZ(123.45)
     DZ3               S              3S 0
     DP31              S              3P 1
     DC3               S              3A
     DS5               S              5A   INZ('98765')
     DJ3               S              3A   INZ('12J')
       c5 = 'ABCDE';
     C                   MOVE      X3            C5
       dsply ('MOVE [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVEL     X3            C5
       dsply ('MOVEL [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVE(P)   X3            C5
       dsply ('MOVEP [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVEL(P)  X3            C5
       dsply ('MOVELP [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVE      P3            C5
       dsply ('MOVE NUM [' + c5 + ']');
     C                   MOVE      S5            Q3
       dsply ('MOVE CHR NUM ' + %char(q3));
     C                   MOVE      J3            Q3
       dsply ('MOVE ZONE SIGN ' + %char(q3));
     C                   MOVE      P52           Z3
       dsply ('MOVE NUM NUM ' + %char(z3));
     C                   MOVEL     P52           P31
       dsply ('MOVEL NUM NUM ' + %char(p31));
     C                   MOVE      N3            C3
       dsply ('MOVE NEG [' + c3 + ']');
       c3 = 'ABC';
     C                   MOVE      7             C3
       dsply ('MOVE LIT [' + c3 + ']');
       c5 = 'ABCDE';
     C                   MOVEL     'QR'          C5
       dsply ('MOVEL LIT [' + c5 + ']');
       *inlr = *on;
