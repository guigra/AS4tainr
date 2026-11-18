      * Stage 2, S2.86: fixed-form SORTA (a character array, a DESCEND
      * packed array, a key overlaying a data structure array: whole
      * elements move) and MOVEA of character data (an array into a
      * field, a field into an array from an index, a literal with and
      * without P). Captured on PUB400 (tools/golden/native86.txt
      * n86_sortmovea) for the native unit NSORTMV.
       ctl-opt dftactgrp(*no);
       dcl-s c3 char(3) dim(4);
       dcl-s n5 packed(5:1) dim(4) descend;
       dcl-ds rows;
         row char(5) dim(3);
         key char(2) overlay(row: 3);
       end-ds;
       dcl-s f12 char(12);
       dcl-s ix packed(3:0) inz(2);
       dcl-s line varchar(60);
       c3(1) = 'pqr';
       c3(2) = 'abc';
       c3(3) = 'xyz';
       c3(4) = 'abd';
       n5(1) = 1.5;
       n5(2) = 30;
       n5(3) = -2;
       n5(4) = 7.2;
       row(1) = 'a1ZZ1';
       row(2) = 'b2AA2';
       row(3) = 'c3MM3';
     C                   SORTA     C3
     C                   SORTA     N5
     C                   SORTA     KEY
       line = 'S ' + c3(1) + c3(2) + c3(3) + c3(4) + ' ' + %char(n5(1))
              + ' ' + %char(n5(4)) + ' ' + rows;
       say(line);
     C                   MOVEA     C3            F12
       line = 'A ' + f12 + '|';
       f12 = '123456789012';
     C                   MOVEA     F12           C3(IX)
       line += c3(1) + c3(2) + c3(3) + c3(4) + '|';
     C                   MOVEA     'XY'          C3(IX)
       line += c3(2) + c3(3) + '|';
     C                   MOVEA(P)  'XY'          C3(IX)
       say(line + c3(2) + c3(3) + c3(4) + '|');
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
