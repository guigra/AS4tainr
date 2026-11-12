      * Stage 2, S2.7: fixed-form DIV and MVR lowered to Rust. Expected:
      * what IBM i 7.5 gave (golden capture tools/golden/native5.txt n5_div,
      * cited in native_units.rs).
     H DFTACTGRP(*NO)
     DA                S              5P 0 INZ(17)
     DB                S              5P 0 INZ(5)
     DQ                S              5P 0
     DR                S              5P 0
     DQ2               S              7P 2
     DR2               S              7P 2
     DN                S              5P 0 INZ(-17)
     DZ                S              5P 0 INZ(0)
     C     A             DIV       B             Q
     C                   MVR                     R
       dsply ('DIV ' + %char(q) + ' MVR ' + %char(r));
     C     A             DIV       B             Q2
     C                   MVR                     R2
       dsply ('DIV2 ' + %char(q2) + ' MVR2 ' + %char(r2));
     C     N             DIV       B             Q
     C                   MVR                     R
       dsply ('DIVNEG ' + %char(q) + ' MVR ' + %char(r));
     C     A             DIV(H)    B             Q
       dsply ('DIVH ' + %char(q));
     C                   DIV       2             A                    505152
       dsply ('DIVRES ' + %char(a) + ' ' + *in50 + *in51 + *in52);
       monitor;
     C     B             DIV       Z             Q
       dsply ('DIV0 FAIL');
       on-error 00102;
       dsply ('DIV0 00102');
       endmon;
       *inlr = *on;
