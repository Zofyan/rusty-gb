fp1 = open("out.txt", "r")
fp2 = open("Blargg7.txt", "r")

lines1 = list(filter(lambda x: len(x) > 30, fp1.readlines()))
lines2 = fp2.readlines()

print(lines1[:10])
for i in range(len(lines2)):
    if lines1[i][:] != lines2[i][:]:
        print("Found different at line %s" % i)
        print("Your line   : %s" % str(lines1[i]))
        print("Correct line: %s" % str(lines2[i]))

        for j in range(10, -1, -1):
            print("%s -------- %s" % (str(lines1[i - j]).strip(), str(lines2[i - j]).strip()))
        exit(0)