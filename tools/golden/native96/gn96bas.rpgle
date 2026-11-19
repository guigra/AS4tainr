      * The review of 2026-11-19: whole-array operations on arrays based
      * on a pointer to other arrays: XFOOT, LOOKUP, SORTA, MOVEA and
      * FOR-EACH see the storage the pointer addresses. Captured on PUB400
      * (tools/golden/native96.txt n96_based) for the native unit NBASARR.
       ctl-opt dftactgrp(*no);
       dcl-s real packed(3:0) dim(4);
       dcl-s p pointer;
       dcl-s arr packed(3:0) dim(4) based(p);
       dcl-s creal char(2) dim(3);
       dcl-s pc pointer;
       dcl-s carr char(2) dim(3) based(pc);
       dcl-s tot packed(5:0);
       dcl-s key packed(3:0) inz(5);
       dcl-s out char(6);
       dcl-s x packed(3:0);
       dcl-s sum packed(5:0);
       dcl-s line varchar(80);
       real(1) = 9;
       real(2) = 5;
       real(3) = 7;
       real(4) = 1;
       p = %addr(real);
     C                   XFOOT     ARR           TOT
     C     KEY           LOOKUP    ARR                                    53
       line = %char(tot) + ' ' + %char(*in53);
       sorta arr;
       line += ' ' + %char(real(1)) + %char(real(2)) + %char(real(3))
               + %char(real(4));
       for-each x in arr;
         sum += x * 2;
       endfor;
       line += ' ' + %char(sum);
       creal(1) = 'ab';
       creal(2) = 'cd';
       creal(3) = 'ef';
       pc = %addr(creal);
     C                   MOVEA     CARR          OUT
       line += ' ' + out;
       say(line);
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
