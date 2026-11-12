      * Stage 2, S2.11: fixed-form MOVE and MOVEL (left, right, padded,
      * character and numeric, the sign from the last zone). Captured on
      * PUB400 (tools/golden/native8.txt n8_move) for the unit test NMOVE.
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
       say('MOVE [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVEL     X3            C5
       say('MOVEL [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVE(P)   X3            C5
       say('MOVEP [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVEL(P)  X3            C5
       say('MOVELP [' + c5 + ']');
       c5 = 'ABCDE';
     C                   MOVE      P3            C5
       say('MOVE NUM [' + c5 + ']');
     C                   MOVE      S5            Q3
       say('MOVE CHR NUM ' + %char(q3));
     C                   MOVE      J3            Q3
       say('MOVE ZONE SIGN ' + %char(q3));
     C                   MOVE      P52           Z3
       say('MOVE NUM NUM ' + %char(z3));
     C                   MOVEL     P52           P31
       say('MOVEL NUM NUM ' + %char(p31));
     C                   MOVE      N3            C3
       say('MOVE NEG [' + c3 + ']');
       c3 = 'ABC';
     C                   MOVE      7             C3
       say('MOVE LIT [' + c3 + ']');
       c5 = 'ABCDE';
     C                   MOVEL     'QR'          C5
       say('MOVEL LIT [' + c5 + ']');
       *inlr = *on;
       // Write one line to gn1.out in the job's current directory (DSPLY
       // under `system` goes to QSYSOPR, a queue PUB400 users share).
       // Fixed-form source: every line ends by column 80.
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s line varchar(200);
         line = text;
         exec sql call qsys2.ifs_write_utf8(
                    path_name => 'gn1.out',
                    line => :line,
                    overwrite => 'APPEND',
                    end_of_line => 'LF');
       end-proc;
