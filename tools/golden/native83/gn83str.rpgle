      * Stage 2, S2.83: fixed-form SCAN (a field, a whole array, a start,
      * a length, not found, a zero length in a field (a literal 0 is
      * RNF5218) with the LO indicator), CHECK,
      * CHECKR, SUBST (with and without P), XLATE and CAT (with and
      * without blanks between, with P, factor 1 left out). Captured on
      * PUB400 (tools/golden/native83.txt n83_strops) for the native unit
      * NSTROPS.
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
       say(line + ' E ' + %char(*in60));
     C     '0123456789'  CHECK     '12A45'       POS
       line = 'C ' + %char(pos);
     C     '0123456789'  CHECKR    'AB123'       POS
       line += ' R ' + %char(pos);
     C     '0123456789'  CHECK     '12A4B'       POSA
       say(line + ' M ' + %char(posa(1)) + %char(posa(2)) + %char(posa(3)));
     C     3             SUBST     'ABCDEFG':2   R10
       line = 'U ' + r10;
     C     3             SUBST(P)  'ABCDEFG':2   R10
       line += ' P ' + r10 + '|';
     C     'abc':'ABC'   XLATE     'aXbYc'       R10
       say(line + ' X ' + r10 + '|');
     C     'AB  '        CAT       'CD'          R10
       line = 'K ' + r10 + '|';
     C     'AB  '        CAT       'CD':1        R10
       line += r10 + '|';
     C                   CAT(P)    'E':0         R10
       say(line + r10 + '|');
       *inlr = *on;
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                  line => :wbuf, overwrite => 'APPEND',
                  end_of_line => 'LF');
       end-proc;
