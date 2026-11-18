      * Native unit NDTAARA (S2.80): IN and OUT of data areas (character,
      * decimal, named by a field, the local data area). The
      * program captured on PUB400, with its data areas NDTAC and NDTAN in
      * QGPL (made by the test as the capture made GN80C and GN80N) and
      * DSPLY for its output.
      * Source: golden capture tools/golden/native80 n80_dtaara (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
       dcl-s txt char(20) dtaara('QGPL/NDTAC');
       dcl-s num packed(7:2) dtaara('QGPL/NDTAN');
       dcl-s dname char(21) inz('QGPL/NDTAC');
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
       dsply (line + ' VAR ' + %trim(via));
       in *lock num;
       line = 'NUM ' + %char(num);
       num += 1.25;
       out num;
       num = 0;
       in num;
       line += ' ' + %char(num);
       txt = *blanks;
       num = 0;
      * The capture's IN *DTAARA: as4tainr's free-form parser takes
      * *DTAARA as the name DTAARA (fidelity.md, interpreter defects), so
      * the two data areas whose values the line shows are read one by one.
       in txt;
       in num;
       dsply (line + ' ALL ' + %trim(txt) + ' ' + %char(num));
       ltxt = 'local';
       out lda;
       ltxt = *blanks;
       in lda;
       dsply ('LDA ' + %trim(ltxt));
       *inlr = *on;
