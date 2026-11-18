      * Stage 2, S2.80: IN and OUT of data areas. A character data area
      * (IN, then IN *LOCK and OUT twice: IBM i ended an OUT with no lock
      * held in RNX9998), a decimal one (IN *LOCK, add, OUT, IN), one
      * named by a field (DTAARA(*VAR:name), in a fixed-form definition:
      * free form takes no *VAR), IN of *DTAARA (every data area of the
      * program), and the local data area as a data structure (OUT, IN;
      * no *LOCK, RNF7090). The data areas GN80C (*CHAR 20 'hello') and
      * GN80N (*DEC 7 2 12.50) are made before the call. Captured on
      * PUB400 (tools/golden/native80.txt n80_dtaara) for the native unit
      * NDTAARA.
       ctl-opt dftactgrp(*no);
       dcl-s txt char(20) dtaara('GUIGRA261/GN80C');
       dcl-s num packed(7:2) dtaara('GUIGRA261/GN80N');
       dcl-s dname char(21) inz('GUIGRA261/GN80C');
     Dvia              S             20    DTAARA(*VAR:dname)
       dcl-ds lda dtaara(*lda);
         ltxt char(10);
       end-ds;
       dcl-s line varchar(52);
       in txt;
       line = 'IN ' + %trim(txt);
       in *lock txt;
       txt = 'changed';
       out txt;
       txt = *blanks;
       in *lock txt;
       line += ' LOCK ' + %trim(txt);
       txt = 'again';
       out txt;
       in via;
       say(line + ' VAR ' + %trim(via));
       in *lock num;
       line = 'NUM ' + %char(num);
       num += 1.25;
       out num;
       num = 0;
       in num;
       line += ' ' + %char(num);
       txt = *blanks;
       num = 0;
       in *dtaara;
       say(line + ' ALL ' + %trim(txt) + ' ' + %char(num));
       ltxt = 'local';
       out lda;
       ltxt = *blanks;
       in lda;
       say('LDA ' + %trim(ltxt));
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
