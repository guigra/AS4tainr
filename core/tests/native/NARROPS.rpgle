      * Native unit NARROPS (S2.85): fixed-form LOOKUP of arrays and
      * XFOOT. The program captured on PUB400, with DSPLY for its output.
      * Source: golden capture tools/golden/native85 n85_arrops (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
       dcl-s arr char(2) dim(5);
       dcl-s num packed(5:2) dim(5) ascend;
       dcl-s i packed(3:0);
       dcl-s tot packed(7:1);
       dcl-s key char(2) inz('CC');
       dcl-s v packed(5:2);
       dcl-s v2 packed(5:2) inz(3.75);
       dcl-s v3 packed(5:2) inz(99);
       dcl-s line varchar(60);
       arr(1) = 'AA';
       arr(2) = 'BB';
       arr(3) = 'CC';
       arr(4) = 'DD';
       arr(5) = 'CC';
       num(1) = 1.25;
       num(2) = 2.50;
       num(3) = 3.75;
       num(4) = 5.05;
       num(5) = 9.99;
       i = 1;
     C     KEY           LOOKUP    ARR(I)                                 53
       line = 'A ' + %char(i) + ' ' +
              %char(*in51) + %char(*in52) + %char(*in53);
       i = 4;
     C     'CC'          LOOKUP    ARR(I)                                 53
       line += ' ' + %char(i) + ' ' +
               %char(*in51) + %char(*in52) + %char(*in53);
       i = 1;
     C     'ZZ'          LOOKUP    ARR(I)                                 53
       line += ' ' + %char(i) + ' ' +
               %char(*in51) + %char(*in52) + %char(*in53);
     C     'DD'          LOOKUP    ARR                                    53
       dsply (line + ' N ' + %char(*in51) + %char(*in52) + %char(*in53));
       v = 3;
       i = 1;
     C     V             LOOKUP    NUM(I)                             51
       line = 'H ' + %char(i) + ' ' +
              %char(*in51) + %char(*in52) + %char(*in53);
       i = 1;
     C     V             LOOKUP    NUM(I)                               52
       line += ' L ' + %char(i) + ' ' +
               %char(*in51) + %char(*in52) + %char(*in53);
       i = 1;
     C     V2            LOOKUP    NUM(I)                             51  53
       line += ' E ' + %char(i) + ' ' +
               %char(*in51) + %char(*in52) + %char(*in53);
       i = 1;
     C     V3            LOOKUP    NUM(I)                             51
       dsply (line + ' X ' + %char(i) + ' ' +
           %char(*in51) + %char(*in52) + %char(*in53));
     C                   XFOOT     NUM           TOT                  515253
       line = 'F ' + %char(tot) + ' ' + 
              %char(*in51) + %char(*in52) + %char(*in53);
     C                   XFOOT(H)  NUM           TOT
       dsply (line + ' ' + %char(tot));
       *inlr = *on;
