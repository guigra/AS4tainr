      * Stage 2, S2.3: fixed-form ADD, SUB, MULT, Z-ADD and Z-SUB lowered to
      * Rust. Expected: what IBM i 7.5 gave for the same statements (golden
      * capture tools/golden/native3.txt n3_arith, cited in native_units.rs).
     H DFTACTGRP(*NO)
     DA                S              5P 2 INZ(12.34)
     DB                S              5P 2 INZ(45.67)
     DR                S              7P 2
     DSMALL            S              3P 0 INZ(998)
     DZ                S              5P 1
     DM                S              7P 3
     C     A             ADD       B             R
       dsply ('ARITH ADD ' + %char(r));
     C     A             SUB       B             R                    505152
       dsply ('ARITH SUB ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   ADD       5             SMALL
       dsply ('ARITH TRUNC ' + %char(small));
     C     A             MULT      B             Z
       dsply ('ARITH MULT ' + %char(z));
     C     A             MULT(H)   B             Z
       dsply ('ARITH MULTH ' + %char(z));
     C     A             MULT      B             M
       dsply ('ARITH MULTFULL ' + %char(m));
     C                   Z-ADD     B             Z                    505152
       dsply ('ARITH ZADD ' + %char(z) + ' ' + *in50 + *in51 + *in52);
     C                   Z-SUB     A             R                    505152
       dsply ('ARITH ZSUB ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   Z-ADD     0             R                    505152
       dsply ('ARITH ZERO ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   SUB       1             R
       dsply ('ARITH SUB1 ' + %char(r));
       *inlr = *on;
