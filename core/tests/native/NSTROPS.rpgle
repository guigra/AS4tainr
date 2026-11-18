      * Native unit NSTROPS (S2.83): fixed-form SCAN, CHECK, CHECKR,
      * SUBST, XLATE and CAT. The program captured on PUB400, with DSPLY
      * for its output.
      * Source: golden capture tools/golden/native83 n83_strops (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
       dcl-s pos packed(3:0);
       dcl-s posa packed(3:0) dim(3);
       dcl-s r10 char(10) inz('0123456789');
       dcl-s base char(10) inz('XXABCXXABC');
       dcl-s st packed(3:0) inz(4);
       dcl-s z packed(3:0) inz(0);
       dcl-s line varchar(60);
     C     'ABC'         SCAN      BASE          POS                      50
       line = 'S ' + %char(pos) + ' ' + %char(*in50);
     C     'ABC'         SCAN      BASE          POSA
       line += ' A ' + %char(posa(1)) + %char(posa(2)) + %char(posa(3));
     C     'ABC'         SCAN      BASE:ST       POS
       line += ' T ' + %char(pos);
     C     'ABD'         SCAN      BASE          POS                      50
       line += ' N ' + %char(pos) + ' ' + %char(*in50);
     C     'ABX':2       SCAN      BASE          POS
       line += ' L ' + %char(pos);
     C     'ABC':Z       SCAN      BASE          POS                    60
       dsply (line + ' E ' + %char(*in60));
     C     '0123456789'  CHECK     '12A45'       POS
       line = 'C ' + %char(pos);
     C     '0123456789'  CHECKR    'AB123'       POS
       line += ' R ' + %char(pos);
     C     '0123456789'  CHECK     '12A4B'       POSA
       dsply (line + ' M ' + %char(posa(1)) + %char(posa(2)) + %char(posa(3)));
     C     3             SUBST     'ABCDEFG':2   R10
       line = 'U ' + r10;
     C     3             SUBST(P)  'ABCDEFG':2   R10
       line += ' P ' + r10 + '|';
     C     'abc':'ABC'   XLATE     'aXbYc'       R10
       dsply (line + ' X ' + r10 + '|');
     C     'AB  '        CAT       'CD'          R10
       line = 'K ' + r10 + '|';
     C     'AB  '        CAT       'CD':1        R10
       line += r10 + '|';
     C                   CAT(P)    'E':0         R10
       dsply (line + r10 + '|');
       *inlr = *on;
