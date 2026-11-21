      * Native unit NEXCDSK (S2.197): EXCEPT to a program-described DISK
      * file, the program captured on PUB400 as GN197ED with DSPLY for its
      * output.
      * Its file is QGPL/NEXCDF, made by the test before the program is
      * compiled, named on the F specification (the capture's program made
      * QTEMP/GN197F itself, named by EXTFILE: the interpreter writes
      * output records only to a file found by its name when the program
      * is compiled; fidelity.md).
      * Source: golden capture tools/golden/native197 n197_exceptdisk
      * (PUB400, IBM i 7.5, 2026-11-21).
     H DFTACTGRP(*NO)
     FNEXCDF    O  A F   20        DISK    USROPN
     D line            S            200    VARYING
     D name            S              8
     D qty             S              5  0
     D got             S             20
      /free
        exec sql set option commit = *none;
        exec sql delete from QGPL/NEXCDF;
        open nexcdf;
        name = 'abc';
        qty = 42;
        except addrec;
        qty = -7;
        except addtwo;
        close nexcdf;
        exec sql declare c1 cursor for
                 select r from QGPL/NEXCDF order by rrn(NEXCDF);
        exec sql open c1;
        exec sql fetch c1 into :got;
        dow sqlcode = 0;
          line += '[' + got + ']';
          exec sql fetch c1 into :got;
        enddo;
        exec sql close c1;
        dsply line;
        *inlr = *on;
      /end-free
     ONEXCDF    EADD         ADDREC
     O                                            1 'T'
     O                       NAME                 9
     O                       QTY                 14
     ONEXCDF    EADD         ADDTWO
     O                                            2 'XY'
     O                       QTY           Z     20
