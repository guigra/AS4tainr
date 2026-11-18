      * Native unit NBITOPS (S2.84): fixed-form TESTN, TESTZ, BITON,
      * BITOFF and TESTB. The program captured on PUB400, with DSPLY for
      * its output.
      * Source: golden capture tools/golden/native84 n84_bitops (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
       dcl-s t3 char(3);
       dcl-s b1 char(1) inz(x'00');
       dcl-s m1 char(1) inz(x'81');
       dcl-s line varchar(60);
       line = 'N';
       t3 = '123';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = ' 12';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '1A3';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '   ';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '12C';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       dsply (line);
       line = 'Z';
       t3 = 'A';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = 'J';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '1';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '&';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '-';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       dsply (line);
       line = 'B';
     C                   BITON     '0167'        B1
     C                   TESTB     '01'          B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   TESTB     '012'         B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   BITOFF    '0'           B1
     C                   TESTB     '0'           B1                   51  53
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   TESTB     M1            B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   BITOFF    M1            B1
     C                   TESTB     '167'         B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       dsply (line);
       *inlr = *on;
